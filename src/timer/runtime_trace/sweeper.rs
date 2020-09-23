use smol::{
    channel::{Receiver, TryRecvError::*},
    future,
    lock::Mutex,
};
use std::{
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd, Reverse},
    collections::BinaryHeap,
    sync::Arc,
};

use super::super::timer_core::{get_timestamp, TimerEvent, TimerEventSender};

#[derive(Default, Eq, Debug)]
pub(crate) struct RecycleUnit {
    deadline: u64,
    task_id: u64,
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

//TODO:reseve.
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

pub(crate) struct RecyclingBins {
    recycle_unit_heap: Mutex<BinaryHeap<Reverse<RecycleUnit>>>,
    recycle_unit_sources: Receiver<RecycleUnit>,
    timer_event_sender: TimerEventSender,
}

//when, event-handle update task_handle then matain recycle_unit_heap.
//TODO: communication by async-channel, t.recv().or(RecyclingBins.excute()).await;
//one by one run.....  maybe don't care time out.
impl RecyclingBins {
    pub(crate) fn new(
        recycle_unit_sources: Receiver<RecycleUnit>,
        timer_event_sender: TimerEventSender,
    ) -> Self {
        let recycle_unit_heap: Mutex<BinaryHeap<Reverse<RecycleUnit>>> =
            Mutex::new(BinaryHeap::new());

        RecyclingBins {
            recycle_unit_heap,
            recycle_unit_sources,
            timer_event_sender,
        }
    }

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

                if let Some(recycle_flag) =
                    (&recycle_unit_heap).peek().map(|r| r.0.deadline <= now)
                {
                    if !recycle_flag {
                        drop(recycle_unit_heap);
                        break;
                    }

                    let recycle_unit = (&mut recycle_unit_heap)
                        .pop()
                        .map(|v| v.0)
                        .unwrap();

                    //handle send-error.
                    self.timer_event_sender
                        .send(TimerEvent::CancelTask(
                            recycle_unit.task_id,
                            recycle_unit.record_id,
                        ))
                        .await
                        .unwrap_or_else(|e| println!("{}", e));

                //send mes to event_handle.

                //TODO: recyle_unit
                } else {
                    drop(recycle_unit_heap);
                    break;
                }
            }

            future::yield_now().await;
            //drop lock.
        }
    }

    pub(crate) async fn add_recycle_unit(self: Arc<Self>) {
        'loopLayer: loop {
            'forLayer: for _ in 0..200 {
                let mut recycle_unit_heap = self.recycle_unit_heap.lock().await;

                match self.recycle_unit_sources.try_recv() {
                    Ok(recycle_unit) => {
                        (&mut recycle_unit_heap).push(dbg!(Reverse(recycle_unit)));
                    }

                    //TODO: Have a Error waiting fot handle.
                    Err(e) => {
                        match e {
                            Empty => {
                                drop(recycle_unit_heap);
                                //TODO: drop.
                                break 'forLayer;
                            }

                            //TODO:if close....
                            Closed => {
                                break 'loopLayer;
                            }
                        }
                    }
                }
            }

            future::yield_now().await;
            // out of scope , recycle_unit_heap is auto-drop;
        }
    }
}
