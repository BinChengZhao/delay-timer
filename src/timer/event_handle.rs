//! A woker for handle events.
//!
//! # EventHandle
//!
//! This is an important entry point to control the flow of tasks:
//!
//! 1. Branch of different mandated events.
//! 2. A communication center for internal and external workers.

pub(crate) use super::super::entity::{SharedHeader, SharedTaskWheel};
use super::runtime_trace::sweeper::{RecycleUnit, RecyclingBins};
use super::runtime_trace::task_handle::TaskTrace;
pub(crate) use super::timer_core::{TimerEvent, DEFAULT_TIMER_SLOT_COUNT};
use super::{Slot, Task, TaskMark};
use crate::prelude::*;

use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::Arc;

use anyhow::Result;
use smol::channel::unbounded;

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

    pub(crate) fn build(self) -> Option<EventHandle> {
        let task_trace = TaskTrace::default();
        let shared_header = self.shared_header?;

        let sub_wokers = SubWorkers::new(
            self.timer_event_sender?,
            shared_header.runtime_instance.kind,
        );

        let timer_event_receiver = self.timer_event_receiver?;
        #[cfg(feature = "status-report")]
        let status_report_sender = self.status_report_sender;

        Some(EventHandle {
            shared_header,
            task_trace,
            timer_event_receiver,
            #[cfg(feature = "status-report")]
            status_report_sender,
            sub_wokers,
        })
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
        async_spawn_by_smol(
            self.sub_wokers
                .recycling_bin_woker
                .inner
                .clone()
                .add_recycle_unit(),
        )
        .detach();
        async_spawn_by_smol(self.sub_wokers.recycling_bin_woker.inner.clone().recycle()).detach();
    }

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

    // handle all event.
    // TODO: Add TestUnit.
    pub(crate) async fn lauch(&mut self) {
        self.init_sub_workers();
        self.handle_event().await;
    }

    fn init_sub_workers(&mut self) {
        let runtime_kind = self.shared_header.runtime_instance.kind;

        match runtime_kind {
            RuntimeKind::Smol => self.recycling_task(),

            RuntimeKind::Tokio => self.recycling_task_by_tokio(),
        };
    }

    async fn handle_event(&mut self) {
        // Turn on `feature` and have `status_report_sender` go this piece of logic.
        #[cfg(feature = "status-report")]
        if let Some(status_report_sender) = self.status_report_sender.take() {
            while let Ok(event) = self.timer_event_receiver.recv().await {
                let public_event_result = PublicEvent::try_from(&event);

                let dispatch_result = self.event_dispatch(event).await;

                match dispatch_result {
                    Ok(event_sync_mark) if event_sync_mark => {
                        if let Ok(public_event) = public_event_result {
                            status_report_sender
                                .send(public_event)
                                .await
                                .unwrap_or_else(|e| error!("event sync error: {}", e));
                        }
                    }
                    Err(e) => {
                        error!("{}", &e);
                    }
                    _ => {}
                }
            }
            return;
        }

        // Did not turn on `feature` or no `status_report_sender` go this piece of logic.
        while let Ok(event) = self.timer_event_receiver.recv().await {
            self.event_dispatch(event)
                .await
                .map_err(|e| error!("{}", e))
                .ok();
        }
    }

    pub(crate) async fn event_dispatch(&mut self, event: TimerEvent) -> Result<bool> {
        trace!("event-dispatch: {:?}", event);

        match event {
            TimerEvent::StopTimer => {
                self.shared_header.shared_motivation.store(false, Release);
                Ok(true)
            }

            TimerEvent::AddTask(task) => self.add_task(task).map(|task_mark| {
                self.record_task_mark(task_mark);
                true
            }),

            TimerEvent::InsertTask(task, task_instances_chain_maintainer) => {
                self.add_task(task).map(|mut task_mark| {
                    task_mark.set_task_instances_chain_maintainer(task_instances_chain_maintainer);
                    self.record_task_mark(task_mark);
                    true
                })
            }

            TimerEvent::UpdateTask(task) => {
                self.update_task(task).await;
                Ok(true)
            }

            TimerEvent::AdvanceTask(task_id) => self.advance_task(task_id).await.map(|_| true),

            TimerEvent::RemoveTask(task_id) => {
                let remove_result = self.remove_task(task_id).await.map(|_| true);

                self.shared_header.task_flag_map.remove(&task_id);
                remove_result
            }
            TimerEvent::CancelTask(task_id, record_id) => {
                self.cancel_task::<true>(task_id, record_id, state::instance::CANCELLED)
            }

            // FIXED:
            // When the `TimeoutTask` event fails to remove the handle, Ok(()) is returned by default.
            // This causes the `TimeoutTask` event to be sent to the outside world by `status_report_sender`,
            // Which is a buggy behavior.

            // Redesign the return value: Result<()> -> Result<bool>
            // Ok(_) & Err(_) for Result, means whether the processing is successful or not.
            // `bool` means whether to synchronize the event to external.
            TimerEvent::TimeoutTask(task_id, record_id) => {
                self.cancel_task::<false>(task_id, record_id, state::instance::TIMEOUT)
            }

            TimerEvent::AppendTaskHandle(task_id, delay_task_handler_box) => {
                self.maintain_task_status(task_id, delay_task_handler_box)
                    .await;
                Ok(true)
            }

            TimerEvent::FinishTask(FinishTaskBody {
                task_id, record_id, ..
            }) => self.finish_task(task_id, record_id),
        }
    }

    pub(crate) async fn send_recycle_unit_sources_sender(&self, recycle_unit: RecycleUnit) {
        self.sub_wokers
            .recycling_bin_woker
            .sender
            .send(recycle_unit)
            .await
            .unwrap_or_else(|e| error!("`send_recycle_unit_sources_sender`: {}", e));
    }

    // Add task to wheel_queue  slot
    fn add_task(&mut self, mut task: Box<Task>) -> AnyResult<TaskMark> {
        let second_hand = self.shared_header.second_hand.current_second_hand();

        let exec_time: u64 = task
            .get_next_exec_timestamp()
            .ok_or_else(|| anyhow!("can't get_next_exec_timestamp in {}", &task.task_id))?;

        let timestamp = self.shared_header.global_time.load(Acquire);

        // Put task on next slot.
        let time_seed: u64 = exec_time
            .checked_sub(timestamp)
            .unwrap_or(task.task_id % DEFAULT_TIMER_SLOT_COUNT)
            + second_hand
            + 1;
        let slot_seed: u64 = time_seed % DEFAULT_TIMER_SLOT_COUNT;

        let cylinder_line = time_seed / DEFAULT_TIMER_SLOT_COUNT;
        task.set_cylinder_line(time_seed / DEFAULT_TIMER_SLOT_COUNT);

        // copy task_id
        let task_id = task.task_id;
        if let Some(mut slot) = self.shared_header.wheel_queue.get_mut(&slot_seed) {
            slot.value_mut().add_task(*task);
        }

        let mut task_mart = TaskMark::default();
        task_mart
            .set_task_id(task_id)
            .set_slot_mark(slot_seed)
            .set_parallel_runnable_num(0);
        debug!(
            "task-id: {} , next-exec-timestamp: {}, slot-seed: {}, cylinder-line: {}",
            task_id, exec_time, slot_seed, cylinder_line
        );

        Ok(task_mart)
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

        if let Some(mut slot) = self.shared_header.wheel_queue.get_mut(&slot_mark) {
            return slot.value_mut().update_task(*task);
        }

        None
    }

    // Take the initiative to perform once Task.
    pub(crate) async fn advance_task(&mut self, task_id: u64) -> Result<()> {
        let task_mark = self
            .shared_header
            .task_flag_map
            .get(&task_id)
            .ok_or_else(|| {
                anyhow!(
                    "Fn : `advance_task`, No task-mark found (task-id: {} )",
                    task_id
                )
            })?;

        let slot_mark = task_mark.value().get_slot_mark();

        let mut task = {
            if let Some(mut slot) = self.shared_header.wheel_queue.get_mut(&slot_mark) {
                slot.value_mut().remove_task(task_id).ok_or_else(|| {
                    anyhow!("Fn : `advance_task`, No task found (task-id: {} )", task_id)
                })?
            } else {
                return Err(anyhow!(
                    "Fn : `advance_task`, No task found (task-id: {} )",
                    task_id
                ));
            }
        };
        task.clear_cylinder_line();

        let slot_seed = self.shared_header.second_hand.current_second_hand() + 1;

        if let Some(mut slot) = self.shared_header.wheel_queue.get_mut(&slot_seed) {
            slot.value_mut().add_task(task);
            return Ok(());
        }

        Err(anyhow!(
            "Fn : `advance_task`, No slot found (slot: {} )",
            slot_seed
        ))
    }

    // for remove task.
    pub(crate) async fn remove_task(&mut self, task_id: u64) -> Result<()> {
        let task_mark = self
            .shared_header
            .task_flag_map
            .get(&task_id)
            .ok_or_else(|| {
                anyhow!(
                    " Fn : `remove_task`,  No task-mark found (task-id: {} )",
                    task_id
                )
            })?;

        let slot_mark = task_mark.value().get_slot_mark();

        if let Some(mut slot) = self.shared_header.wheel_queue.get_mut(&slot_mark) {
            slot.value_mut().remove_task(task_id);
            return Ok(());
        }

        Err(anyhow!(
            "Fn : `remove_task`, No slot found (slot: {} )",
            slot_mark
        ))
    }

    // The `INITIATIVE` mark indicates whether the cancellation was initiated by an outside party.

    // `INITIATIVE` = true
    // External initiative to cancel the action,
    // If the cancellation fails then the error log record needs to be kept.

    // `INITIATIVE` = false
    // Passive cancellation at runtime (e.g., timeout) indicates that
    // The task instance has completed or has been actively cancelled,
    // And no error logging is required.
    pub(crate) fn cancel_task<const INITIATIVE: bool>(
        &mut self,
        task_id: u64,
        record_id: i64,
        state: usize,
    ) -> Result<bool> {
        // The cancellation operation is executed first, and then the outside world is notified of the cancellation event.
        // If the operation object does not exist in the middle, it should return early.

        let quit_result = self.task_trace.quit_one_task_handler(task_id, record_id);

        if quit_result.is_err() {
            if INITIATIVE {
                quit_result?;
            } else {
                return Ok(false);
            }
        }

        if let Some(mut task_mark_ref_mut) = self.shared_header.task_flag_map.get_mut(&task_id) {
            let task_mark = task_mark_ref_mut.value_mut();

            task_mark.dec_parallel_runnable_num();

            if task_mark.task_instances_chain_maintainer.is_some() {
                // Here the user can be notified that the task instance has disappeared via `Instance`.
                task_mark.notify_cancel_finish(record_id, state)?;
            }

            return Ok(true);
        }

        if INITIATIVE {
            Err(anyhow!(
                "Fn : `cancel_task`, Without the `task_mark_ref_mut` for task_id :{}, record_id : {}, state : {}",
                task_id,
                record_id,
                state
            ))
        } else {
            Ok(false)
        }
    }

    pub(crate) fn finish_task(&mut self, task_id: u64, record_id: i64) -> Result<bool> {
        // `task-handler` should exit first regardless of whether `task_mark_ref_mut` exists or not.
        self.task_trace.quit_one_task_handler(task_id, record_id)?;

        if let Some(mut task_mark_ref_mut) = self.shared_header.task_flag_map.get_mut(&task_id) {
            let task_mark = task_mark_ref_mut.value_mut();

            if task_mark.task_instances_chain_maintainer.is_some() {
                // Here the user can be notified that the task instance has disappeared via `Instance`.
                task_mark.notify_cancel_finish(record_id, state::instance::COMPLETED)?;
            }

            task_mark.dec_parallel_runnable_num();

            return Ok(true);
        }

        error!(
            "Fn : `finish_task`, Without the `task_mark_ref_mut` for task_id :{}, record_id : {}",
            task_id, record_id
        );

        Ok(true)
    }

    pub(crate) async fn maintain_task_status(
        &mut self,
        task_id: u64,
        delay_task_handler_box: DelayTaskHandlerBox,
    ) {
        {
            if let Some(mut task_mark) = self.shared_header.task_flag_map.get_mut(&task_id) {
                let task_instances_chain_maintainer_option =
                    task_mark.value_mut().get_task_instances_chain_maintainer();

                if let Some(task_instances_chain_maintainer) =
                    task_instances_chain_maintainer_option
                {
                    let instance = Instance::default()
                        .set_task_id(task_id)
                        .set_record_id(delay_task_handler_box.get_record_id());

                    task_instances_chain_maintainer
                        .push_instance(instance)
                        .await;
                }
            } else {
                error!("Missing task_mark for task_id : {}", task_id)
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
        let task_wheel = DashMap::new();

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
    fn new(timer_event_sender: TimerEventSender, runtime_kind: RuntimeKind) -> Self {
        let recycling_bin_woker = RecyclingBinWorker::new(timer_event_sender, runtime_kind);

        SubWorkers {
            recycling_bin_woker,
        }
    }
}

impl RecyclingBinWorker {
    fn new(timer_event_sender: TimerEventSender, runtime_kind: RuntimeKind) -> Self {
        let (recycle_unit_sources_sender, recycle_unit_sources_reciver) =
            unbounded::<RecycleUnit>();

        let inner = Arc::new(RecyclingBins::new(
            recycle_unit_sources_reciver,
            timer_event_sender,
            runtime_kind,
        ));

        RecyclingBinWorker {
            inner,
            sender: recycle_unit_sources_sender,
        }
    }
}
