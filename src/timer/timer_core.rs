//! Timer-core
//! It is the core of the entire cycle scheduling task.
use crate::prelude::*;

use crate::entity::get_timestamp;
use crate::entity::RuntimeKind;

use std::mem::replace;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::time::Duration;
use std::time::Instant;

use smol::Timer as smolTimer;

pub(crate) const DEFAULT_TIMER_SLOT_COUNT: u64 = 3600;

/// The clock of timer core.
struct Clock {
    inner: ClockInner,
}

enum ClockInner {
    Sc(SmolClock),
    #[cfg(feature = "tokio-support")]
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
            #[cfg(feature = "tokio-support")]
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
            #[cfg(feature = "tokio-support")]
            ClockInner::Tc(ref mut tokio_clock) => tokio_clock.tick().await,
        }
    }
}
cfg_tokio_support!(
    use tokio::time::{Interval, interval_at, self};

    #[derive(Debug)]
    struct TokioClock{
        inner : Interval
    }

    impl TokioClock{

        pub fn new(start: time::Instant, period: Duration) -> Self{
            let inner = interval_at(start, period);
            TokioClock{inner}
        }

        pub async fn tick(&mut self){
            self.inner.tick().await;
        }
    }

);

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
#[derive(Clone, Debug)]
/// delay-timer internal timer wheel core.
pub struct Timer {
    /// Event sender that provides events to `EventHandle` processing.
    pub(crate) timer_event_sender: TimerEventSender,
    status_report_sender: Option<AsyncSender<i32>>,
    pub(crate) shared_header: SharedHeader,
}

// In any case, the task is not executed in the Scheduler,
// and task-Fn determines which runtime to put the internal task in when it is generated.
// just provice api and struct ,less is more.
impl Timer {
    /// Initialize a timer wheel core.
    pub fn new(timer_event_sender: TimerEventSender, shared_header: SharedHeader) -> Self {
        Timer {
            timer_event_sender,
            status_report_sender: None,
            shared_header,
        }
    }

    //TODO:cfg macro
    #[cfg(feature = "status_report")]
    pub(crate) fn set_status_report_sender(&mut self, sender: AsyncSender<i32>) {
        self.status_report_sender = Some(sender);
    }

    /// Offset the current slot by one when reading it,
    /// so event_handle can be easily inserted into subsequent slots.
    pub(crate) fn next_position(&mut self) -> u64 {
        self.shared_header
            .second_hand
            .fetch_update(Release, Relaxed, |x| {
                Some((x + 1) % DEFAULT_TIMER_SLOT_COUNT)
            })
            .unwrap_or_else(|e| e)
    }

    /// Return a future can pool it for Schedule all cycles task.
    pub(crate) async fn async_schedule(&mut self) {
        // if that overtime , i run it not block
        let mut second_hand;
        let mut next_second_hand;
        let mut timestamp;

        let runtime_kind = self.shared_header.runtime_instance.kind;
        let mut clock = Clock::new(runtime_kind);

        loop {
            //TODO: replenish ending single, for stop current jod and thread.
            if !self.shared_header.shared_motivation.load(Acquire) {
                return;
            }

            second_hand = self.next_position();
            timestamp = get_timestamp();
            self.shared_header.global_time.store(timestamp, Release);
            let task_ids;

            {
                if let Some(mut slot_mut) = self.shared_header.wheel_queue.get_mut(&second_hand) {
                    task_ids = slot_mut.value_mut().arrival_time_tasks();
                } else {
                    error!("Missing data for wheel slot {}.", second_hand);
                    continue;
                }
            }

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
                    next_second_hand = (second_hand + 1) % DEFAULT_TIMER_SLOT_COUNT;
                    self.maintain_task(task, timestamp, next_second_hand)
                        .await
                        .map_err(|e| error!("{}", e))
                        .ok();
                }
            }

            clock.tick().await;
        }
    }

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

            //if runnable_task.parallel_runnable_num >= task.maximum_parallel_runnable_num doesn't run it.

            if parallel_runnable_num >= maximum_parallel_runnable_num {
                return self.handle_task(task, timestamp, next_second_hand, false);
            }
        }

        let mut task_context = TaskContext::default();
        task_context
            .task_id(task_id)
            .record_id(record_id)
            .timer_event_sender(self.timer_event_sender.clone());

        let task_handler_box = (task.get_body())(task_context);

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

        // Time difference + next second hand % DEFAULT_TIMER_SLOT_COUNT
        let step = task_excute_timestamp.checked_sub(timestamp).unwrap_or(1) + next_second_hand;
        let quan = step / DEFAULT_TIMER_SLOT_COUNT;
        task.set_cylinder_line(quan);
        let slot_seed = step % DEFAULT_TIMER_SLOT_COUNT;

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
        Ok(())
    }
}

mod tests {

    #[test]
    fn test_next_position() {
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
            .store(3599, Ordering::SeqCst);
        assert_eq!(timer.next_position(), 3599);
        assert_eq!(timer.next_position(), 0);
    }
}
