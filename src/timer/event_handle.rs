//! A woker for handle events.
//!
//! # EventHandle
//!
//! This is an important entry point to control the flow of tasks:
//!
//! 1. Branch of different mandated events.
//! 2. A communication center for internal and external workers.

pub(crate) use super::{
    super::delay_timer::{SharedHeader, SharedTaskWheel},
    runtime_trace::{
        sweeper::{RecycleUnit, RecyclingBins},
        task_handle::TaskTrace,
    },
    timer_core::{
        Slot, Task, TaskMark, TimerEvent, TimerEventReceiver, TimerEventSender,
        DEFAULT_TIMER_SLOT_COUNT,
    },
};

use crate::{AsyncReceiver, AsyncSender};
use anyhow::Result;
use std::sync::{
    atomic::Ordering::{Acquire, Release},
    Arc,
};
use waitmap::WaitMap;

use smol::{
    channel::{unbounded, Sender},
    future::{block_on, FutureExt},
};
use std::thread::spawn as thread_spawn;
cfg_tokio_support!(
    use tokio::sync::mpsc::unbounded_channel;
);

//TaskTrace: use event mes update.
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
    //Data Senders for Resource Recyclers.
    pub(crate) recycle_unit_sources_sender: AsyncSender<RecycleUnit>,
    //Shared header information.
    pub(crate) shared_header: SharedHeader,
}

impl EventHandle {
    pub(crate) fn new(
        timer_event_receiver: TimerEventReceiver,
        timer_event_sender: TimerEventSender,
        shared_header: SharedHeader,
    ) -> Self {
        let status_report_sender: Option<AsyncSender<i32>> = None;
        let task_trace = TaskTrace::default();

        let (recycle_unit_sources_sender, recycle_unit_sources_reciver) =
            Self::recycle_unit_sources_channel();

        let recycling_bins = Arc::new(RecyclingBins::new(
            recycle_unit_sources_reciver,
            timer_event_sender,
        ));

        //TODO: need spawn in runtime or single thread because tokio just can spawn in runtime.
        //not use race, use commone futures::join or others.
        //mutex
        //TODO:optimize.
        smol::spawn(
            recycling_bins
                .clone()
                .add_recycle_unit()
                .race(recycling_bins.recycle()),
        )
        .detach();

        EventHandle {
            task_trace,
            timer_event_receiver,
            status_report_sender,
            recycle_unit_sources_sender,
            shared_header,
        }
    }

    //TODO:分多个 impl 去 给类型实现方法
    cfg_tokio_support!(
        fn recycle_unit_sources_channel() -> (AsyncSender<RecycleUnit>, AsyncReceiver<RecycleUnit>)
        {
            unbounded_channel::<RecycleUnit>()
        }
    );

    #[cfg(not(feature = "tokio-support"))]
    fn recycle_unit_sources_channel() -> (AsyncSender<RecycleUnit>, AsyncReceiver<RecycleUnit>) {
        unbounded::<RecycleUnit>()
    }

    #[allow(dead_code)]
    pub(crate) fn set_status_report_sender(&mut self, status_report_sender: AsyncSender<i32>) {
        self.status_report_sender = Some(status_report_sender);
    }

    //handle all event.
    //TODO:Add TestUnit.
    #[cfg(not(feature = "tokio-support"))]
    pub(crate) async fn handle_event(&mut self) {
        while let Ok(event) = self.timer_event_receiver.recv().await {
            self.event_dispatch(event).await;
        }
    }
    cfg_tokio_support!(
        pub(crate) async fn handle_event(&mut self) {
            while let Some(event) = self.timer_event_receiver.recv().await {
                self.event_dispatch(event).await;
            }
        }
    );

    pub(crate) async fn event_dispatch(&mut self, event: TimerEvent) {
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
        }
    }

    #[cfg(not(feature = "tokio-support"))]
    pub(crate) async fn send_recycle_unit_sources_sender(&self, recycle_unit: RecycleUnit) {
        self.recycle_unit_sources_sender
            .send(recycle_unit)
            .await
            .unwrap_or_else(|e| println!("{}", e));
    }

    cfg_tokio_support!(
        pub(crate) async fn send_recycle_unit_sources_sender(&self, recycle_unit: RecycleUnit) {
            self.recycle_unit_sources_sender
                .send(recycle_unit)
                .unwrap_or_else(|e| println!("{}", e));
        }
    );

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

        TaskMark::new(task_id, slot_seed)
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
