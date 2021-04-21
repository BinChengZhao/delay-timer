//! A woker for handle events.
//!
//! # EventHandle
//!
//! This is an important entry point to control the flow of tasks:
//!
//! 1. Branch of different mandated events.
//! 2. A communication center for internal and external workers.

pub(crate) use super::super::entity::{SharedHeader, SharedTaskWheel};
use super::runtime_trace::{
    sweeper::{RecycleUnit, RecyclingBins},
    task_handle::TaskTrace,
};
pub(crate) use super::timer_core::{TimerEvent, DEFAULT_TIMER_SLOT_COUNT};
use super::{Slot, Task, TaskMark};

use crate::prelude::*;
use anyhow::Result;
use smol::channel::unbounded;
use std::sync::{
    atomic::Ordering::{Acquire, Release},
    Arc,
};
use waitmap::WaitMap;

cfg_status_report!(
    use std::convert::TryFrom;
    type StatusReportSender = Option<AsyncSender<PublicEvent>>;
);
#[derive(Debug, Default, Clone)]
pub(crate) struct EventHandleBuilder {
    //Shared header information.
    pub(crate) shared_header: Option<SharedHeader>,
    //The core of the event recipient, dealing with the global event.
    pub(crate) timer_event_receiver: Option<TimerEventReceiver>,
    pub(crate) timer_event_sender: Option<TimerEventSender>,
    #[warn(dead_code)]
    #[cfg(feature = "status-report")]
    pub(crate) status_report_sender: StatusReportSender,
}

impl EventHandleBuilder {
    pub(crate) fn timer_event_receiver(
        &mut self,
        timer_event_receiver: TimerEventReceiver,
    ) -> &mut Self {
        self.timer_event_receiver = Some(timer_event_receiver);
        self
    }

    pub(crate) fn timer_event_sender(&mut self, timer_event_sender: TimerEventSender) -> &mut Self {
        self.timer_event_sender = Some(timer_event_sender);
        self
    }

    pub(crate) fn shared_header(&mut self, shared_header: SharedHeader) -> &mut Self {
        self.shared_header = Some(shared_header);
        self
    }

    pub(crate) fn build(self) -> EventHandle {
        let task_trace = TaskTrace::default();
        let sub_wokers = SubWorkers::new(self.timer_event_sender.unwrap());

        let timer_event_receiver = self.timer_event_receiver.unwrap();
        let shared_header = self.shared_header.unwrap();
        #[cfg(feature = "status-report")]
        let status_report_sender = self.status_report_sender;

        EventHandle {
            shared_header,
            task_trace,
            timer_event_receiver,
            sub_wokers,
            #[cfg(feature = "status-report")]
            status_report_sender,
        }
    }
}

/// New a instance of EventHandle.
///
/// The parameter `timer_event_receiver` is used by EventHandle
/// to accept all internal events.
///
/// event may come from user application or sub-worker.
///
/// The parameter `timer_event_sender` is used by sub-workers
/// report processed events.
///
/// The paramete `shared_header` is used to shared delay-timer core data.
// TaskTrace: use event mes update.
// remove Task, can't stop runing taskHandle, just though cancel or cancelAll with taskid.
// maybe cancelAll msg before last `update msg`  check the
// flag_map slotid with biggest task-slotid in trace, if has one delay, send a msg for recycleer
// let it to trash the last taskhandle.
pub(crate) struct EventHandle {
    //Shared header information.
    pub(crate) shared_header: SharedHeader,
    //Task Handle Collector, which makes it easy to cancel a running task.
    pub(crate) task_trace: TaskTrace,
    //The core of the event recipient, dealing with the global event.
    pub(crate) timer_event_receiver: TimerEventReceiver,
    #[cfg(feature = "status-report")]
    pub(crate) status_report_sender: StatusReportSender,
    //The sub-workers of EventHandle.
    pub(crate) sub_wokers: SubWorkers,
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

    //handle all event.
    //TODO:Add TestUnit.
    pub(crate) async fn lauch(&mut self) {
        self.init_sub_workers();
        self.handle_event().await;
    }

    fn init_sub_workers(&mut self) {
        let runtime_kind = self.shared_header.runtime_instance.kind;

        match runtime_kind {
            RuntimeKind::Smol => self.recycling_task(),
            #[cfg(feature = "tokio-support")]
            RuntimeKind::Tokio => self.recycling_task_by_tokio(),
        };
    }

    async fn handle_event(&mut self) {
        #[cfg(feature = "status-report")]
        if let Some(status_report_sender) = self.status_report_sender.take() {
            while let Ok(event) = self.timer_event_receiver.recv().await {
                if let Ok(public_event) = PublicEvent::try_from(&event) {
                    status_report_sender
                        .send(public_event)
                        .await
                        .unwrap_or_else(|e| print!("{}", e));
                }
                self.event_dispatch(event).await;
            }
            return;
        }

        while let Ok(event) = self.timer_event_receiver.recv().await {
            self.event_dispatch(event).await;
        }
    }

    pub(crate) async fn event_dispatch(&mut self, event: TimerEvent) {
        match event {
            TimerEvent::StopTimer => {
                self.shared_header.shared_motivation.store(false, Release);
                return;
            }
            TimerEvent::AddTask(task) => {
                let task_mark = self.add_task(task);
                self.record_task_mark(task_mark);
            }

            TimerEvent::InsertTask(task, task_instances_chain_maintainer) => {
                let mut task_mark = self.add_task(task);
                task_mark.set_task_instances_chain_maintainer(task_instances_chain_maintainer);

                self.record_task_mark(task_mark);
            }

            TimerEvent::UpdateTask(task) => {
                self.update_task(task).await;
            }

            TimerEvent::RemoveTask(task_id) => {
                self.remove_task(task_id).await;

                //FIXME: cancel maybe doesn't execute drop.
                self.shared_header.task_flag_map.cancel(&task_id);
            }
            TimerEvent::CancelTask(task_id, record_id) => {
                self.cancel_task(task_id, record_id);
            }

            TimerEvent::AppendTaskHandle(task_id, delay_task_handler_box) => {
                self.maintain_task_status(task_id, delay_task_handler_box)
                    .await;
            }

            TimerEvent::FinishTask(task_id, record_id, _finish_time) => {
                //TODO: maintain a outside-task-handle , through it pass the _finish_time and final-state.
                // Provide a separate start time for the external, record_id time with a delay.
                // Or use snowflake.real_time to generate record_id , so you don't have to add a separate field.
                self.finish_task(task_id, record_id);
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

    // Add task to wheel_queue  slot
    fn add_task(&mut self, mut task: Box<Task>) -> TaskMark {
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
            .add_task(*task);

        let mut task_mart = TaskMark::default();
        task_mart
            .set_task_id(task_id)
            .set_slot_mark(slot_seed)
            .set_parallel_runable_num(0);

        task_mart
    }

    // for record task-mark.
    pub(crate) fn record_task_mark(&mut self, task_mark: TaskMark) {
        self.shared_header
            .task_flag_map
            .insert(task_mark.task_id, task_mark);
    }

    // for update task.
    pub(crate) async fn update_task(&mut self, task: Box<Task>) -> Option<Task> {
        let task_mark = self.shared_header.task_flag_map.get(&task.task_id)?;

        let slot_mark = task_mark.value().get_slot_mark();

        self.shared_header
            .wheel_queue
            .get_mut(&slot_mark)
            .unwrap()
            .value_mut()
            .update_task(*task)
    }

    // for remove task.
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

    pub(crate) fn cancel_task(&mut self, task_id: u64, record_id: i64) -> Option<Result<()>> {
        let mut task_mark_ref_mut = self.shared_header.task_flag_map.get_mut(&task_id).unwrap();
        let task_mark = task_mark_ref_mut.value_mut();

        task_mark.dec_parallel_runable_num();

        // Here the user can be notified that the task instance has disappeared via `Instance`.
        task_mark.notify_cancel_finish(record_id, state::instance::CANCELLED);

        self.task_trace.quit_one_task_handler(task_id, record_id)
    }

    pub(crate) fn finish_task(&mut self, task_id: u64, record_id: i64) -> Option<Result<()>> {
        let mut task_mark_ref_mut = self.shared_header.task_flag_map.get_mut(&task_id).unwrap();
        let task_mark = task_mark_ref_mut.value_mut();

        task_mark.dec_parallel_runable_num();

        // Here the user can be notified that the task instance has disappeared via `Instance`.
        task_mark.notify_cancel_finish(record_id, state::instance::COMPLETED);

        self.task_trace.quit_one_task_handler(task_id, record_id)
    }

    pub(crate) async fn maintain_task_status(
        &mut self,
        task_id: u64,
        delay_task_handler_box: DelayTaskHandlerBox,
    ) {
        {
            let mut task_mark = self.shared_header.task_flag_map.get_mut(&task_id).unwrap();

            let task_instances_chain_maintainer_option =
                task_mark.value_mut().get_task_instances_chain_maintainer();

            if let Some(task_instances_chain_maintainer) = task_instances_chain_maintainer_option {
                let instance = Instance::default()
                    .set_task_id(task_id)
                    .set_record_id(delay_task_handler_box.get_record_id());

                task_instances_chain_maintainer
                    .push_instance(instance)
                    .await;
            }
        }

        // If has deadline, set recycle_unit.
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

    pub(crate) fn init_task_wheel(slots_numbers: u64) -> SharedTaskWheel {
        let task_wheel = WaitMap::new();

        for i in 0..slots_numbers {
            task_wheel.insert(i, Slot::new());
        }

        Arc::new(task_wheel)
    }
}

cfg_status_report!(
impl EventHandleBuilder {
    pub(crate) fn status_report_sender(
        &mut self,
        status_report_sender: AsyncSender<PublicEvent>,
    ) -> &mut Self {
        self.status_report_sender = Some(status_report_sender);
        self
    }
}
);

impl SubWorkers {
    fn new(timer_event_sender: TimerEventSender) -> Self {
        let recycling_bin_woker = RecyclingBinWorker::new(timer_event_sender);

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
