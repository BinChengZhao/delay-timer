use crate::prelude::{get_timestamp, yield_now, AsyncMutex, AsyncReceiver};

cfg_tokio_support!(
    use tokio::sync::mpsc::error::TryRecvError::*;
);

cfg_smol_support!(
    use smol::channel::TryRecvError::*;
);
use std::{
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd, Reverse},
    collections::BinaryHeap,
    sync::Arc,
};

use super::super::timer_core::{TimerEvent, TimerEventSender};

#[derive(Default, Eq, Debug, Copy, Clone)]
/// recycle unit.
pub(crate) struct RecycleUnit {
    /// deadline.
    deadline: u64,

    /// task-id.
    task_id: u64,

    ///record-id.
    record_id: i64,
}

impl RecycleUnit {
    pub(crate) fn new(deadline: u64, task_id: u64, record_id: i64) -> Self {
        RecycleUnit {
            deadline,
            task_id,
            record_id,
        }
    }
}

impl Ord for RecycleUnit {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline.cmp(&other.deadline)
    }
}

impl PartialOrd for RecycleUnit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for RecycleUnit {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}

#[derive(Debug)]
///RecyclingBins is resource recycler, excute timeout task-handle.
pub(crate) struct RecyclingBins {
    ///storage all task-handle in there.
    recycle_unit_heap: AsyncMutex<BinaryHeap<Reverse<RecycleUnit>>>,

    /// use it to recieve source-data build recycle-unit.
    recycle_unit_sources: AsyncMutex<AsyncReceiver<RecycleUnit>>,

    /// notify timeout-event to event-handler for cancel that.
    timer_event_sender: TimerEventSender,
}

impl RecyclingBins {
    /// construct.
    pub(crate) fn new(
        recycle_unit_sources: AsyncReceiver<RecycleUnit>,
        timer_event_sender: TimerEventSender,
    ) -> Self {
        let recycle_unit_heap: AsyncMutex<BinaryHeap<Reverse<RecycleUnit>>> =
            AsyncMutex::new(BinaryHeap::new());
        let recycle_unit_sources = AsyncMutex::new(recycle_unit_sources);

        RecyclingBins {
            recycle_unit_heap,
            recycle_unit_sources,
            timer_event_sender,
        }
    }

    /// alternate run fn between recycle and  add_recycle_unit.
    pub(crate) async fn recycle(self: Arc<Self>) {
        //get now timestamp

        //peek inside RecyclingBins has anyone meet contitions.

        //pop one of recycle_unit_heap, execute it.

        loop {
            let mut recycle_unit_heap = self.recycle_unit_heap.lock().await;

            let now: u64 = get_timestamp();
            for _ in 0..200 {
                // recv from channel, if no item in channel
                // drop lock.

                if let Some(recycle_flag) = (&recycle_unit_heap).peek().map(|r| r.0.deadline <= now)
                {
                    if !recycle_flag {
                        drop(recycle_unit_heap);
                        break;
                    }

                    let recycle_unit = (&mut recycle_unit_heap).pop().map(|v| v.0).unwrap();

                    //handle send-error.
                    self.send_timer_event(TimerEvent::CancelTask(
                        recycle_unit.task_id,
                        recycle_unit.record_id,
                    ))
                    .await;

                //send msg to event_handle.
                } else {
                    drop(recycle_unit_heap);
                    break;
                }
            }

            yield_now().await;
            //drop lock.
        }
    }

    #[cfg(not(feature = "tokio-support"))]
    pub(crate) async fn send_timer_event(&self, event: TimerEvent) {
        self.timer_event_sender
            .send(event)
            .await
            .unwrap_or_else(|e| println!("{}", e));
    }

    cfg_tokio_support!(
        pub(crate) async fn send_timer_event(&self, event: TimerEvent) {
            self.timer_event_sender
                .send(event)
                .unwrap_or_else(|e| println!("{}", e));
        }
    );
    /// alternate run fn between recycle and  add_recycle_unit.
    pub(crate) async fn add_recycle_unit(self: Arc<Self>) {
        'loopLayer: loop {
            'forLayer: for _ in 0..200 {
                let mut recycle_unit_heap = self.recycle_unit_heap.lock().await;

                match self.recycle_unit_sources.lock().await.try_recv() {
                    Ok(recycle_unit) => {
                        (&mut recycle_unit_heap).push(Reverse(recycle_unit));
                    }

                    Err(e) => match e {
                        Empty => {
                            drop(recycle_unit_heap);
                            break 'forLayer;
                        }

                        Closed => {
                            drop(recycle_unit_heap);
                            break 'loopLayer;
                        }
                    },
                }
            }

            yield_now().await;
            // out of scope , recycle_unit_heap is auto-drop;
        }
    }
}

mod tests {

    #[test]
    fn test_task_valid() {
        use super::{get_timestamp, RecycleUnit, RecyclingBins, TimerEvent};
        use smol::{
            block_on,
            channel::{unbounded, TryRecvError},
            future::FutureExt,
        };
        use std::{
            sync::Arc,
            thread::{park_timeout, spawn as thread_spawn},
            time::Duration,
        };

        let (timer_event_sender, timer_event_receiver) = unbounded::<TimerEvent>();
        let (recycle_unit_sender, recycle_unit_receiver) = unbounded::<RecycleUnit>();

        let recycling_bins = Arc::new(RecyclingBins::new(
            recycle_unit_receiver,
            timer_event_sender,
        ));
        //TODO:optimize.
        thread_spawn(move || {
            block_on(async {
                recycling_bins
                    .clone()
                    .recycle()
                    .or(recycling_bins.add_recycle_unit())
                    .await;
            })
        });

        let deadline = get_timestamp() + 5;

        for i in 1..10 {
            recycle_unit_sender
                .try_send(RecycleUnit::new(deadline, i, (i * i) as i64))
                .unwrap();
        }

        park_timeout(Duration::new(2, 0));

        if let Err(e) = timer_event_receiver.try_recv() {
            assert_eq!(e, TryRecvError::Empty);
        }
        park_timeout(Duration::new(3, 3_000_000));

        for _ in 1..10 {
            assert!(dbg!(timer_event_receiver.try_recv()).is_ok());
        }
    }
}
