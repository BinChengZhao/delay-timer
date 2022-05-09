//! Timer-core
//! It is the core of the entire cycle scheduling task.
use crate::prelude::*;

use crate::entity::timestamp;
use crate::entity::RuntimeKind;

use std::mem::replace;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::time::Duration;
use std::time::Instant;

use smol::Timer as smolTimer;

pub(crate) const DEFAULT_TIMER_SLOT_COUNT: u64 = 3600;

/// The clock of timer core.
#[derive(Debug)]
struct Clock {
    inner: ClockInner,
}

#[derive(Debug)]
enum ClockInner {
    Sc(SmolClock),

    Tc(TokioClock),
}

impl Clock {
    fn new(runtime_kind: RuntimeKind) -> Clock {
        let inner = ClockInner::new(runtime_kind);
        Clock { inner }
    }
}
impl ClockInner {
    fn new(runtime_kind: RuntimeKind) -> ClockInner {
        match runtime_kind {
            RuntimeKind::Smol => {
                ClockInner::Sc(SmolClock::new(Instant::now(), Duration::from_secs(1)))
            }

            RuntimeKind::Tokio => ClockInner::Tc(TokioClock::new(
                time::Instant::now(),
                Duration::from_secs(1),
            )),
        }
    }
}

impl Clock {
    async fn tick(&mut self) {
        match self.inner {
            ClockInner::Sc(ref mut smol_clock) => smol_clock.tick().await,

            ClockInner::Tc(ref mut tokio_clock) => tokio_clock.tick().await,
        };
    }
}
use tokio::time::{self, interval_at, Interval};

#[derive(Debug)]
struct TokioClock {
    inner: Interval,
}

impl TokioClock {
    pub fn new(start: time::Instant, period: Duration) -> Self {
        let inner = interval_at(start, period);
        TokioClock { inner }
    }

    pub async fn tick(&mut self) {
        self.inner.tick().await;
    }
}

#[derive(Debug)]
struct SmolClock {
    inner: smolTimer,
    period: Duration,
    offset: Instant,
}

impl SmolClock {
    pub fn new(start: Instant, period: Duration) -> Self {
        let offset = start + period;
        let inner = smolTimer::at(offset);
        SmolClock {
            inner,
            period,
            offset,
        }
    }

    pub async fn tick(&mut self) {
        self.offset += self.period;
        let new_inner = smolTimer::at(self.offset);
        replace(&mut self.inner, new_inner).await;
    }
}

/// The information generated when completing a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishTaskBody {
    pub(crate) task_id: u64,
    pub(crate) record_id: i64,
    pub(crate) finish_time: u64,
    pub(crate) finish_output: Option<FinishOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// The output generated when the task is completed.
pub enum FinishOutput {
    /// The output generated when the process task is completed.
    ProcessOutput(std::process::Output),
    /// Exception output for a task that did not run successfully.
    ExceptionOutput(String),
}

//warning: large size difference between variants
/// Event for Timer Wheel Core.
#[derive(Debug)]
pub enum TimerEvent {
    /// Stop the Timer.
    StopTimer,
    /// Add a new `Task`.
    AddTask(Box<Task>),
    /// Insert a new `Task`.
    /// Maintain a state that is transparent to the user, such as the end of a task running instance.
    InsertTask(Box<Task>, TaskInstancesChainMaintainer),
    /// Update a Task in Timer .
    UpdateTask(Box<Task>),
    /// Remove a Task in Timer .
    RemoveTask(u64),
    /// Cancel a Task running instance in Timer .
    CancelTask(u64, i64),
    /// Cancel a timeout Task running instance in Timer .
    TimeoutTask(u64, i64),
    /// Finished a Task running instance in Timer .
    FinishTask(FinishTaskBody),
    /// Append a new instance of a running task .
    AppendTaskHandle(u64, DelayTaskHandlerBox),
    /// Take the initiative to perform once Task.
    AdvanceTask(u64),
}
#[derive(Debug)]
/// delay-timer internal timer wheel core.
pub struct Timer {
    /// Event sender that provides events to `EventHandle` processing.
    timer_event_sender: TimerEventSender,
    #[allow(dead_code)]
    status_report_sender: Option<AsyncSender<i32>>,
    shared_header: SharedHeader,
    clock: Clock,
}

// In any case, the task is not executed in the Scheduler,
// and task-Fn determines which runtime to put the internal task in when it is generated.
// just provice api and struct ,less is more.
impl Timer {
    /// Initialize a timer wheel core.
    pub fn new(timer_event_sender: TimerEventSender, shared_header: SharedHeader) -> Self {
        let runtime_kind = shared_header.runtime_instance.kind;
        let clock = Clock::new(runtime_kind);

        Timer {
            timer_event_sender,
            status_report_sender: None,
            shared_header,
            clock,
        }
    }

    #[cfg(feature = "status_report")]
    pub(crate) fn set_status_report_sender(&mut self, sender: AsyncSender<i32>) {
        self.status_report_sender = Some(sender);
    }

    /// Offset the current slot by one when reading it,
    /// so event_handle can be easily inserted into subsequent slots.
    pub(crate) fn next_position(&mut self) -> u64 {
        self.shared_header.second_hand.next().unwrap_or_else(|e| e)
    }

    /// Time goes on, the clock ticks.
    pub(crate) async fn lapse(&mut self) {
        self.clock.tick().await;
        self.next_position();
    }

    /// Return a future can pool it for Schedule all cycles task.
    pub(crate) async fn async_schedule(&mut self) {
        // if that overtime , run it not block

        let mut second_hand = self.second_hand();
        let mut next_second_hand = second_hand + 1;
        let mut current_timestamp = timestamp();

        loop {
            //TODO: replenish ending single, for stop current jod and thread.
            if !self.shared_header.shared_motivation.load(Acquire) {
                return;
            }

            self.shared_header
                .global_time
                .store(current_timestamp, Release);
            let task_ids;

            {
                // TODO:
                // Attempt to batch take out tasks and execute them,
                // marking the status for this batch.
                //
                // Attempt to re-queue the `cancel` event into the channel
                // if the user `cancel` task is running.
                if let Some(mut slot_mut) = self.shared_header.wheel_queue.get_mut(&second_hand) {
                    task_ids = slot_mut.value_mut().arrival_time_tasks();
                } else {
                    error!("Missing data for wheel slot {}.", second_hand);
                    continue;
                }
            }

            trace!(
                "second_hand: {}, timestamp: {}, task_ids: {:?}",
                second_hand,
                current_timestamp,
                task_ids
            );

            // Centralize task processing to avoid duplicate lock requests and releases.
            // FIXME: https://github.com/BinChengZhao/delay-timer/issues/29
            for task_id in task_ids {
                let task_option: Option<Task>;

                {
                    if let Some(mut slot_mut) = self.shared_header.wheel_queue.get_mut(&second_hand)
                    {
                        task_option = slot_mut.value_mut().remove_task(task_id);
                    } else {
                        task_option = None;
                    }
                }

                if let Some(task) = task_option {
                    self.maintain_task(task, current_timestamp, next_second_hand)
                        .await
                        .map_err(|e| error!("{}", e))
                        .ok();
                }
            }

            {
                // When the operation is finished with the task, shrink the container in time
                // To avoid the overall time-wheel from occupying too much memory.
                if let Some(mut slot_mut) = self.shared_header.wheel_queue.get_mut(&second_hand) {
                    slot_mut.shrink();
                }
            }

            self.lapse().await;

            second_hand = self.second_hand();
            next_second_hand = (second_hand + 1) % DEFAULT_TIMER_SLOT_COUNT;
            current_timestamp = timestamp();
        }
    }

    /// Access to the second-hand
    pub(crate) fn second_hand(&self) -> u64 {
        self.shared_header.second_hand.current_second_hand()
    }

    /// Send timer-event to event-handle.
    pub(crate) async fn send_timer_event(
        &mut self,
        task_id: u64,
        tmp_task_handler_box: DelayTaskHandlerBox,
    ) {
        self.timer_event_sender
            .send(TimerEvent::AppendTaskHandle(task_id, tmp_task_handler_box))
            .await
            .unwrap_or_else(|e| error!(" `send_timer_event`: {}", e));
    }

    #[inline(always)]
    /// Maintain the running status of task.
    pub async fn maintain_task(
        &mut self,
        mut task: Task,
        timestamp: u64,
        next_second_hand: u64,
    ) -> AnyResult<()> {
        let record_id: i64 = self
            .shared_header
            .id_generator
            .lock()
            .await
            .real_time_generate();
        let task_id: u64 = task.task_id;

        if let Some(maximum_parallel_runnable_num) = task.maximum_parallel_runnable_num {
            let parallel_runnable_num: u64;

            {
                let task_flag_map =
                    self.shared_header
                        .task_flag_map
                        .get(&task_id)
                        .ok_or_else(|| {
                            anyhow!("Can't get task_flag_map for task : {}", task.task_id)
                        })?;
                parallel_runnable_num = task_flag_map.value().get_parallel_runnable_num();
            }

            // if runnable_task.parallel_runnable_num >= task.maximum_parallel_runnable_num doesn't run it.

            if parallel_runnable_num >= maximum_parallel_runnable_num {
                trace!("task-id: {}, parallel_runnable_num >= maximum_parallel_runnable_num doesn't run it", task.task_id);
                return self.handle_task(task, timestamp, next_second_hand, false);
            }
        }

        let mut task_context = TaskContext::default();
        task_context
            .task_id(task_id)
            .record_id(record_id)
            .timer_event_sender(self.timer_event_sender.clone())
            .runtime_kind(self.shared_header.runtime_instance.kind);

        let task_handler_box = self.routine_exec(&*(task.routine.0), task_context);

        let delay_task_handler_box_builder = DelayTaskHandlerBoxBuilder::default();
        let tmp_task_handler_box = delay_task_handler_box_builder
            .set_task_id(task_id)
            .set_record_id(record_id)
            .set_start_time(timestamp)
            .set_end_time(task.get_maximum_running_time(timestamp))
            .spawn(task_handler_box);

        self.send_timer_event(task_id, tmp_task_handler_box).await;

        let task_valid = task.down_count_and_set_vaild();
        if !task_valid {
            return Ok(());
        }

        self.handle_task(task, timestamp, next_second_hand, true)
    }

    // Use `next_second_hand` to solve a problem
    // (when exec_timestamp - timestamp = 0, a task that needs to be executed immediately
    // is instead put on the next turn)
    pub(crate) fn handle_task(
        &mut self,
        mut task: Task,
        timestamp: u64,
        next_second_hand: u64,
        update_runnable_num: bool,
    ) -> AnyResult<()> {
        let task_id: u64 = task.task_id;

        // Next execute timestamp.
        let task_excute_timestamp = task
            .get_next_exec_timestamp()
            .ok_or_else(|| anyhow!("can't get_next_exec_timestamp in task :{}", task.task_id))?;

        // cylinder_line = 24
        // slot_seed = 60
        // when-init: slot_seed+=1 == 61
        // when-on-slot61-exec: (task_excute_timestamp - timestamp + next_second_hand) % slot_seed == 61

        // Time difference + next second hand % DEFAULT_TIMER_SLOT_COUNT
        let step = task_excute_timestamp.checked_sub(timestamp).unwrap_or(1);
        let cylinder_line = step / DEFAULT_TIMER_SLOT_COUNT;
        task.set_cylinder_line(cylinder_line);
        let slot_seed = (step + next_second_hand) % DEFAULT_TIMER_SLOT_COUNT;

        {
            let mut slot_mut = self
                .shared_header
                .wheel_queue
                .get_mut(&slot_seed)
                .ok_or_else(|| anyhow!("can't slot_mut for slot :{}", slot_seed))?;

            slot_mut.value_mut().add_task(task);
        }

        {
            let mut task_flag_map = self
                .shared_header
                .task_flag_map
                .get_mut(&task_id)
                .ok_or_else(|| anyhow!("can't get task_flag_map for task :{}", task_id))?;

            task_flag_map.value_mut().set_slot_mark(slot_seed);
            if update_runnable_num {
                task_flag_map.value_mut().inc_parallel_runnable_num();
            }
        }

        debug!(
            "task-id: {} , next-exec-timestamp: {}, slot-seed: {}, cylinder-line: {}",
            task_id, task_excute_timestamp, slot_seed, cylinder_line
        );
        Ok(())
    }

    #[inline(always)]
    fn routine_exec(
        &self,
        routine: &(dyn Routine<TokioHandle = TokioJoinHandle<()>, SmolHandle = SmolJoinHandler<()>>
              + 'static
              + Send),

        task_context: TaskContext,
    ) -> Box<dyn DelayTaskHandler> {
        match task_context.runtime_kind {
            RuntimeKind::Smol => create_delay_task_handler(routine.spawn_by_smol(task_context)),
            RuntimeKind::Tokio => create_delay_task_handler(routine.spawn_by_tokio(task_context)),
        }
    }
}

mod tests {

    #[tokio::test]
    async fn test_next_position() {
        use super::{SharedHeader, Timer, TimerEvent};
        use smol::channel::unbounded;
        use std::sync::atomic::Ordering;
        let (s, _) = unbounded::<TimerEvent>();
        let mut timer = Timer::new(s, SharedHeader::default());

        assert_eq!(timer.next_position(), 0);
        assert_eq!(timer.next_position(), 1);

        timer
            .shared_header
            .second_hand
            .inner
            .store(3599, Ordering::SeqCst);
        assert_eq!(timer.next_position(), 3599);
        assert_eq!(timer.next_position(), 0);
    }
}
