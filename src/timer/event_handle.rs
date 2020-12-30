//! A woker for handle events.
//!
//! # EventHandle
//!
//! This is an important entry point to control the flow of tasks:
//!
//! 1. Branch of different mandated events.
//! 2. A communication center for internal and external workers.

pub(crate) use super::{
    super::entity::{SharedHeader, SharedTaskWheel},
    runtime_trace::{
        sweeper::{RecycleUnit, RecyclingBins},
        task_handle::TaskTrace,
    },
    timer_core::{Slot, Task, TaskMark, TimerEvent, DEFAULT_TIMER_SLOT_COUNT},
};

use crate::prelude::*;
use anyhow::Result;
use smol::channel::unbounded;
use std::sync::{
    atomic::Ordering::{Acquire, Release},
    Arc,
};
use waitmap::WaitMap;

// TaskTrace: use event mes update.
// remove Task, can't stop runing taskHandle, just though cancel or cancelAll with taskid.
// maybe cancelAll msg before last `update msg`  check the
// flag_map slotid with biggest task-slotid in trace, if has one delay, send a msg for recycleer
// let it to trash the last taskhandle.
pub(crate) struct EventHandle {
    //Task Handle Collector, which makes it easy to cancel a running task.
    pub(crate) task_trace: TaskTrace,
    //The core of the event recipient, dealing with the global event.
    pub(crate) timer_event_receiver: TimerEventReceiver,
    //TODO:Reporter.
    #[warn(dead_code)]
    pub(crate) status_report_sender: Option<AsyncSender<i32>>,
    //The sub-workers of EventHandle.
    pub(crate) sub_wokers: SubWorkers,
    //Shared header information.
    pub(crate) shared_header: SharedHeader,
}

/// These sub-workers are the left and right arms of `EventHandle`
/// and are responsible for helping it maintain global events.
pub(crate) struct SubWorkers {
    recycling_bin_woker: RecyclingBinWorker,
}

pub(crate) struct RecyclingBinWorker {
    inner: Arc<RecyclingBins>,
    //Data Senders for Resource Recyclers.
    sender: AsyncSender<RecycleUnit>,
}

impl EventHandle {
    /// New a instance of EventHandle.
    ///
    /// The first parameter `timer_event_receiver` is used by EventHandle
    /// to accept all internal events.
    ///
    /// event may come from user application or sub-worker.
    ///
    /// The second parameter `timer_event_sender` is used by sub-workers
    /// report processed events.
    ///
    /// The thrid parameter is used to shared delay-timer core data.
    pub(crate) fn new(
        timer_event_receiver: TimerEventReceiver,
        timer_event_sender: TimerEventSender,
        shared_header: SharedHeader,
    ) -> Self {
        let status_report_sender: Option<AsyncSender<i32>> = None;
        let task_trace = TaskTrace::default();
        let sub_wokers = SubWorkers::new(timer_event_sender);

        EventHandle {
            task_trace,
            timer_event_receiver,
            status_report_sender,
            sub_wokers,
            shared_header,
        }
    }

    fn recycling_task(&mut self) {
        async_spawn(
            self.sub_wokers
                .recycling_bin_woker
                .inner
                .clone()
                .add_recycle_unit(),
        )
        .detach();
        async_spawn(self.sub_wokers.recycling_bin_woker.inner.clone().recycle()).detach();
    }

    cfg_tokio_support!(
        // `async_spawn_by_tokio` 'must be called from the context of Tokio runtime configured
        // with either `basic_scheduler` or `threaded_scheduler`'.
        fn recycling_task_by_tokio(&mut self) {
            async_spawn_by_tokio(
                self.sub_wokers
                    .recycling_bin_woker
                    .inner
                    .clone()
                    .add_recycle_unit(),
            );
            async_spawn_by_tokio(self.sub_wokers.recycling_bin_woker.inner.clone().recycle());
        }
    );

    #[allow(dead_code)]
    pub(crate) fn set_status_report_sender(&mut self, status_report_sender: AsyncSender<i32>) {
        self.status_report_sender = Some(status_report_sender);
    }

    //handle all event.
    //TODO:Add TestUnit.
    pub(crate) async fn handle_event(&mut self) {
        self.init_sub_workers();

        while let Ok(event) = self.timer_event_receiver.recv().await {
            self.event_dispatch(event).await;
        }
    }

    fn init_sub_workers(&mut self) {
        let runtime_kind = self.shared_header.runtime_instance.kind;

        match runtime_kind {
            RuntimeKind::Smol => self.recycling_task(),
            #[cfg(feature = "tokio-support")]
            RuntimeKind::Tokio => self.recycling_task_by_tokio(),
        };
    }

    pub(crate) async fn event_dispatch(&mut self, event: TimerEvent) {
        //#[cfg(features="status-report")]
        //And event isn't `AddTask`, use channel sent(event) to report_channel.
        //defined a new outside-event support user.

        match event {
            TimerEvent::StopTimer => {
                self.shared_header.shared_motivation.store(false, Release);
                return;
            }
            TimerEvent::AddTask(task) => {
                let task_mark = self.add_task(*task);
                self.record_task_mark(task_mark);
            }
            TimerEvent::RemoveTask(task_id) => {
                self.remove_task(task_id).await;
                self.shared_header.task_flag_map.cancel(&task_id);
            }
            TimerEvent::CancelTask(task_id, record_id) => {
                self.cancel_task(task_id, record_id);
            }

            TimerEvent::AppendTaskHandle(task_id, delay_task_handler_box) => {
                //if has deadline, set recycle_unit.
                if let Some(deadline) = delay_task_handler_box.get_end_time() {
                    let recycle_unit = RecycleUnit::new(
                        deadline,
                        delay_task_handler_box.get_task_id(),
                        delay_task_handler_box.get_record_id(),
                    );
                    self.send_recycle_unit_sources_sender(recycle_unit).await;
                }

                self.task_trace.insert(task_id, delay_task_handler_box);
            }

            TimerEvent::FinishTask(task_id, record_id) => {
                dbg!(task_id, record_id, "FinishTask");
                self.cancel_task(task_id, record_id);
            }
        }
    }

    pub(crate) async fn send_recycle_unit_sources_sender(&self, recycle_unit: RecycleUnit) {
        self.sub_wokers
            .recycling_bin_woker
            .sender
            .send(recycle_unit)
            .await
            .unwrap_or_else(|e| println!("{}", e));
    }

    //TODO:
    //cancel for exit running task.
    //stop is suspension of execution(set vaild).
    //user delete task , node should remove.
    //any `Task` can set `valid` for that stop.

    //add task to wheel_queue  slot
    fn add_task(&mut self, mut task: Task) -> TaskMark {
        let second_hand = self.shared_header.second_hand.load(Acquire);
        let exec_time: u64 = task.get_next_exec_timestamp();
        let timestamp = self.shared_header.global_time.load(Acquire);

        let time_seed: u64 = exec_time
            .checked_sub(timestamp)
            .unwrap_or_else(|| task.task_id % DEFAULT_TIMER_SLOT_COUNT)
            + second_hand;
        let slot_seed: u64 = time_seed % DEFAULT_TIMER_SLOT_COUNT;

        task.set_cylinder_line(time_seed / DEFAULT_TIMER_SLOT_COUNT);

        //copu task_id
        let task_id = task.task_id;
        self.shared_header
            .wheel_queue
            .get_mut(&slot_seed)
            .unwrap()
            .value_mut()
            .add_task(task);

        TaskMark::new(task_id, slot_seed, 1)
    }

    //for record task-mark.
    pub(crate) fn record_task_mark(&mut self, task_mark: TaskMark) {
        self.shared_header
            .task_flag_map
            .insert(task_mark.task_id, task_mark);
    }

    //for remove task.
    pub(crate) async fn remove_task(&mut self, task_id: u64) -> Option<Task> {
        let task_mark = self.shared_header.task_flag_map.get(&task_id)?;

        let slot_mark = task_mark.value().get_slot_mark();

        self.shared_header
            .wheel_queue
            .get_mut(&slot_mark)
            .unwrap()
            .value_mut()
            .remove_task(task_id)
    }

    pub fn cancel_task(&mut self, task_id: u64, record_id: i64) -> Option<Result<()>> {
        self.shared_header
            .task_flag_map
            .get_mut(&task_id)
            .unwrap()
            .value_mut()
            .dec_parallel_runable_num();

        self.task_trace.quit_one_task_handler(task_id, record_id)
    }

    pub(crate) fn init_task_wheel(slots_numbers: u64) -> SharedTaskWheel {
        let task_wheel = WaitMap::new();

        for i in 0..slots_numbers {
            task_wheel.insert(i, Slot::new());
        }

        Arc::new(task_wheel)
    }
}

impl SubWorkers {
    fn new(timer_event_sender: TimerEventSender) -> Self {
        let recycling_bin_woker = RecyclingBinWorker::new(timer_event_sender.clone());

        SubWorkers {
            recycling_bin_woker,
        }
    }
}

impl RecyclingBinWorker {
    fn new(timer_event_sender: TimerEventSender) -> Self {
        let (recycle_unit_sources_sender, recycle_unit_sources_reciver) =
            unbounded::<RecycleUnit>();

        let inner = Arc::new(RecyclingBins::new(
            recycle_unit_sources_reciver,
            timer_event_sender,
        ));

        RecyclingBinWorker {
            inner,
            sender: recycle_unit_sources_sender,
        }
    }
}
