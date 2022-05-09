//! Task
//! It is a basic periodic task execution unit.
use super::runtime_trace::task_handle::DelayTaskHandler;
use crate::prelude::*;

use std::cell::RefCell;
use std::fmt;
use std::fmt::Pointer;
use std::str::FromStr;
use std::sync::atomic::Ordering;

use cron_clock::{Schedule, ScheduleIteratorOwned, Utc};
use lru::LruCache;

// Parsing cache for cron expressions, stored with thread-local storage.
thread_local!(static CRON_EXPRESSION_CACHE: RefCell<LruCache<ScheduleIteratorTimeZoneQuery, DelayTimerScheduleIteratorOwned>> = RefCell::new(LruCache::new(256)));

// TaskMark is used to maintain the status of running tasks.
#[derive(Default, Debug)]
pub(crate) struct TaskMark {
    // The id of task.
    pub(crate) task_id: u64,
    // The wheel slot where the task is located.
    slot_mark: u64,
    // Number of tasks running in parallel.
    parallel_runnable_num: u64,
    /// Chain of task run instances.
    /// For inner maintain to Running-Task's instance.
    pub(crate) task_instances_chain_maintainer: Option<TaskInstancesChainMaintainer>,
}

impl TaskMark {
    #[inline(always)]
    pub(crate) fn set_task_id(&mut self, task_id: u64) -> &mut Self {
        self.task_id = task_id;
        self
    }

    #[inline(always)]
    pub(crate) fn get_slot_mark(&self) -> u64 {
        self.slot_mark
    }

    #[inline(always)]
    pub(crate) fn set_slot_mark(&mut self, slot_mark: u64) -> &mut Self {
        self.slot_mark = slot_mark;
        self
    }

    #[inline(always)]
    pub(crate) fn get_parallel_runnable_num(&self) -> u64 {
        self.parallel_runnable_num
    }

    #[inline(always)]
    pub(crate) fn set_parallel_runnable_num(&mut self, parallel_runnable_num: u64) -> &mut Self {
        debug!(
            "task-id: {}, parallel_runnable_num: {}",
            self.task_id, self.parallel_runnable_num
        );
        self.parallel_runnable_num = parallel_runnable_num;
        self
    }

    #[inline(always)]
    pub(crate) fn inc_parallel_runnable_num(&mut self) {
        let parallel_runnable_num = self.parallel_runnable_num + 1;
        self.set_parallel_runnable_num(parallel_runnable_num);
    }

    #[inline(always)]
    pub(crate) fn dec_parallel_runnable_num(&mut self) {
        let parallel_runnable_num = self
            .parallel_runnable_num
            .checked_sub(1)
            .unwrap_or_default();

        self.set_parallel_runnable_num(parallel_runnable_num);
    }

    #[inline(always)]
    pub(crate) fn set_task_instances_chain_maintainer(
        &mut self,
        task_instances_chain_maintainer: TaskInstancesChainMaintainer,
    ) -> &mut Self {
        self.task_instances_chain_maintainer = Some(task_instances_chain_maintainer);
        self
    }

    pub(crate) fn get_task_instances_chain_maintainer(
        &mut self,
    ) -> Option<&mut TaskInstancesChainMaintainer> {
        let state = self
            .task_instances_chain_maintainer
            .as_ref()
            .map(|c| c.inner_state.load(Ordering::Acquire));

        if state == Some(state::instance_chain::DROPPED) {
            self.task_instances_chain_maintainer = None;
        }

        self.task_instances_chain_maintainer.as_mut()
    }

    pub(crate) fn notify_cancel_finish(
        &mut self,
        record_id: i64,
        state: usize,
    ) -> AnyResult<Instance> {
        let task_instances_chain_maintainer = self.get_task_instances_chain_maintainer().ok_or_else(|| {
            anyhow!(
                "Fn : `notify_cancel_finish`, No task-instances-chain-maintainer found (record-id: {} , state : {} )",
                record_id, state
            )
        })?;

        let index = task_instances_chain_maintainer
            .inner_list
            .iter()
            .position(|d| d.get_record_id() == record_id)
            .ok_or_else(|| anyhow!("No task-handle-index found ( record-id: {} )", record_id))?;

        let mut has_remove_instance_list =
            task_instances_chain_maintainer.inner_list.split_off(index);

        let remove_instance = has_remove_instance_list
            .pop_front()
            .ok_or_else(|| anyhow!("No task-handle found in list ( record-id: {} )", record_id))?;

        task_instances_chain_maintainer
            .inner_list
            .append(&mut has_remove_instance_list);

        remove_instance.notify_cancel_finish(state);

        Ok(remove_instance)
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum FrequencyUnify<'a> {
    FrequencyCronStr(FrequencyCronStr<'a>),
    FrequencySeconds(FrequencySeconds),
}

impl<'a> Default for FrequencyUnify<'a> {
    fn default() -> FrequencyUnify<'a> {
        FrequencyUnify::FrequencySeconds(FrequencySeconds::default())
    }
}

#[derive(Debug, Copy, Clone)]
/// Enumerated values of repeating types based on the string of cron-expression.
pub enum FrequencyCronStr<'a> {
    /// Repeat once.
    Once(&'a str),
    /// Repeat ad infinitum.
    Repeated(&'a str),
    /// Type of countdown.
    CountDown(u64, &'a str),
}

#[derive(Debug, Copy, Clone)]
/// Enumerated values of repeating types based on the number of seconds.
pub(crate) enum FrequencySeconds {
    /// Repeat once.
    Once(u64),
    /// Repeat ad infinitum.
    Repeated(u64),
    /// Type of countdown.
    CountDown(u64, u64),
}

impl<'a> Default for FrequencyCronStr<'a> {
    fn default() -> FrequencyCronStr<'a> {
        FrequencyCronStr::Once("@minutely")
    }
}

impl Default for FrequencySeconds {
    fn default() -> FrequencySeconds {
        FrequencySeconds::Once(ONE_MINUTE)
    }
}

/// Iterator for task internal control of execution time.
#[derive(Debug, Clone)]
pub(crate) enum FrequencyInner {
    /// Unlimited repetition types for cron-expression.
    CronExpressionRepeated(DelayTimerScheduleIteratorOwned),
    /// Type of countdown for cron-expression.
    CronExpressionCountDown(u64, DelayTimerScheduleIteratorOwned),
    /// Unlimited repetition types for seconds-duration.
    SecondsRepeated(SecondsState),
    /// Type of countdown for SecondsState.
    /// SecondsCountDown(count_down, SecondsState)
    SecondsCountDown(u64, SecondsState),
}

impl<'a> TryFrom<(FrequencyUnify<'a>, ScheduleIteratorTimeZone)> for FrequencyInner {
    type Error = FrequencyAnalyzeError;

    fn try_from(
        (frequency, time_zone): (FrequencyUnify<'_>, ScheduleIteratorTimeZone),
    ) -> Result<FrequencyInner, Self::Error> {
        let frequency_inner = match frequency {
            FrequencyUnify::FrequencyCronStr(FrequencyCronStr::Once(cron_str)) => {
                let task_schedule =
                    DelayTimerScheduleIteratorOwned::analyze_cron_expression(time_zone, cron_str)?;

                FrequencyInner::CronExpressionCountDown(1, task_schedule)
            }
            FrequencyUnify::FrequencyCronStr(FrequencyCronStr::Repeated(cron_str)) => {
                let task_schedule =
                    DelayTimerScheduleIteratorOwned::analyze_cron_expression(time_zone, cron_str)?;

                FrequencyInner::CronExpressionRepeated(task_schedule)
            }
            FrequencyUnify::FrequencyCronStr(FrequencyCronStr::CountDown(count_down, cron_str)) => {
                let task_schedule =
                    DelayTimerScheduleIteratorOwned::analyze_cron_expression(time_zone, cron_str)?;

                FrequencyInner::CronExpressionCountDown(count_down as u64, task_schedule)
            }

            FrequencyUnify::FrequencySeconds(FrequencySeconds::Once(seconds)) => {
                if seconds == 0 {
                    return Err(FrequencyAnalyzeError::DisInitTime);
                }

                let seconds_state: SecondsState =
                    ((timestamp() + seconds)..).step_by(seconds as usize);
                FrequencyInner::SecondsCountDown(1, seconds_state)
            }
            FrequencyUnify::FrequencySeconds(FrequencySeconds::Repeated(seconds)) => {
                if seconds == 0 {
                    return Err(FrequencyAnalyzeError::DisInitTime);
                }

                let seconds_state: SecondsState =
                    ((timestamp() + seconds)..).step_by(seconds as usize);

                FrequencyInner::SecondsRepeated(seconds_state)
            }
            FrequencyUnify::FrequencySeconds(FrequencySeconds::CountDown(count_down, seconds)) => {
                if seconds == 0 {
                    return Err(FrequencyAnalyzeError::DisInitTime);
                }

                let seconds_state: SecondsState =
                    ((timestamp() + seconds)..).step_by(seconds as usize);
                FrequencyInner::SecondsCountDown(count_down, seconds_state)
            }
        };

        Ok(frequency_inner)
    }
}

/// Set the time zone for the time of the expression iteration.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ScheduleIteratorTimeZone {
    /// Utc specifies the UTC time zone. It is most efficient.
    Utc,
    /// Local specifies the system local time zone.
    Local,
    /// FixedOffset specifies an arbitrary, fixed time zone such as UTC+09:00 or UTC-10:30. This often results from the parsed textual date and time. Since it stores the most information and does not depend on the system environment, you would want to normalize other TimeZones into this type.
    FixedOffset(FixedOffset),
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub(crate) struct ScheduleIteratorTimeZoneQuery {
    time_zone: ScheduleIteratorTimeZone,
    cron_expression: String,
}

impl ScheduleIteratorTimeZone {
    fn get_fixed_offset(&self) -> AnyResult<FixedOffset> {
        match self {
            ScheduleIteratorTimeZone::FixedOffset(offset) => Ok(*offset),
            _ => Err(anyhow!("No variant of FixedOffset.")),
        }
    }
}

impl Default for ScheduleIteratorTimeZone {
    fn default() -> Self {
        ScheduleIteratorTimeZone::Local
    }
}

/// The Cron-expression scheduling iterator enum.
/// There are three variants.
/// The declaration `enum` is to avoid the problems caused by generalized contagion and monomorphism.
///
//
// Frequency<T> -> FrequencyInner<T> -> Task<T> -> Slot<T> -> Wheel<T> ....
// Frequency<Utc> or Frequency<Local> caused Task<Utc> Task<Local>
// The Wheel<T> must only exist one for delay-timer run ,
// can't store two kind of task-type .
//
///
/// The intention is to provide an api to the user to set the time zone of `ScheduleIteratorOwned` conveniently,
/// if you use a generic that wraps its type need to add this generic parameter,
/// and after the type will be inconsistent and can not be stored in the same container,
/// so use enum to avoid these problems.

#[derive(Debug, Clone)]
pub(crate) enum DelayTimerScheduleIteratorOwned {
    Utc(ScheduleIteratorOwned<Utc>),
    Local(ScheduleIteratorOwned<Local>),
    FixedOffset(ScheduleIteratorOwned<FixedOffset>),
}

impl DelayTimerScheduleIteratorOwned {
    pub(crate) fn new(
        ScheduleIteratorTimeZoneQuery {
            time_zone,
            ref cron_expression,
        }: ScheduleIteratorTimeZoneQuery,
    ) -> Result<DelayTimerScheduleIteratorOwned, cron_error::Error> {
        Ok(match time_zone {
            ScheduleIteratorTimeZone::Utc => DelayTimerScheduleIteratorOwned::Utc(
                Schedule::from_str(cron_expression)?.upcoming_owned(Utc),
            ),
            ScheduleIteratorTimeZone::Local => DelayTimerScheduleIteratorOwned::Local(
                Schedule::from_str(cron_expression)?.upcoming_owned(Local),
            ),
            ScheduleIteratorTimeZone::FixedOffset(fixed_offset) => {
                DelayTimerScheduleIteratorOwned::FixedOffset(
                    Schedule::from_str(cron_expression)?.upcoming_owned(fixed_offset),
                )
            }
        })
    }

    #[inline(always)]
    pub(crate) fn refresh_previous_datetime(&mut self, time_zone: ScheduleIteratorTimeZone) {
        match self {
            Self::Utc(ref mut iterator) => iterator.refresh_previous_datetime(Utc),
            Self::Local(ref mut iterator) => iterator.refresh_previous_datetime(Local),

            Self::FixedOffset(ref mut iterator) => {
                if let Ok(offset) = time_zone.get_fixed_offset() {
                    iterator.refresh_previous_datetime(offset);
                }
            }
        }
    }

    #[inline(always)]
    pub(crate) fn next(&mut self) -> Option<i64> {
        match self {
            Self::Utc(ref mut iterator) => iterator.next().map(|e| e.timestamp()),
            Self::Local(ref mut iterator) => iterator.next().map(|e| e.timestamp()),
            Self::FixedOffset(ref mut iterator) => iterator.next().map(|e| e.timestamp()),
        }
    }

    // Analyze expressions, get cache.
    fn analyze_cron_expression(
        time_zone: ScheduleIteratorTimeZone,
        cron_expression: &str,
    ) -> Result<DelayTimerScheduleIteratorOwned, FrequencyAnalyzeError> {
        let indiscriminate_expression = cron_expression.trim_matches(' ').to_owned();
        let schedule_iterator_time_zone_query: ScheduleIteratorTimeZoneQuery =
            ScheduleIteratorTimeZoneQuery {
                cron_expression: indiscriminate_expression,
                time_zone,
            };

        let analyze_result = CRON_EXPRESSION_CACHE.try_with(|expression_cache| {
            let mut lru_cache = expression_cache.borrow_mut();
            if let Some(schedule_iterator) = lru_cache.get(&schedule_iterator_time_zone_query) {
                let mut schedule_iterator_copy = schedule_iterator.clone();

                // Reset the internal base time to avoid expiration time during internal iterations.
                schedule_iterator_copy.refresh_previous_datetime(time_zone);

                return Ok(schedule_iterator_copy);
            }

            let new_result =
                DelayTimerScheduleIteratorOwned::new(schedule_iterator_time_zone_query.clone());

            new_result.map(|task_schedule| {
                lru_cache.put(schedule_iterator_time_zone_query, task_schedule.clone());
                task_schedule
            })
        })?;

        Ok(analyze_result?)
    }
}

impl FrequencyInner {
    // How many times the acquisition needs to be performed.
    #[allow(dead_code)]
    fn residual_time(&self) -> u64 {
        match self {
            FrequencyInner::CronExpressionRepeated(_) => u64::MAX,
            FrequencyInner::SecondsRepeated(_) => u64::MAX,
            FrequencyInner::CronExpressionCountDown(ref time, _) => *time,
            FrequencyInner::SecondsCountDown(ref time, _) => *time,
        }
    }

    fn next_alarm_timestamp(&mut self) -> Option<i64> {
        match self {
            FrequencyInner::CronExpressionCountDown(_, ref mut clock) => clock.next(),
            FrequencyInner::CronExpressionRepeated(ref mut clock) => clock.next(),
            FrequencyInner::SecondsRepeated(seconds_state) => {
                seconds_state.next().map(|s| s as i64)
            }
            FrequencyInner::SecondsCountDown(_, seconds_state) => {
                seconds_state.next().map(|s| s as i64)
            }
        }
    }

    #[warn(unused_parens)]
    fn down_count(&mut self) {
        match self {
            FrequencyInner::CronExpressionRepeated(_) => {}
            FrequencyInner::SecondsRepeated(_) => {}
            FrequencyInner::CronExpressionCountDown(ref mut exec_count, _) => *exec_count -= 1u64,
            FrequencyInner::SecondsCountDown(count_down, _) => *count_down -= 1u64,
        };
    }

    fn is_down_over(&self) -> bool {
        matches!(
            self,
            FrequencyInner::CronExpressionCountDown(0, _) | FrequencyInner::SecondsCountDown(0, _)
        )
    }
}

//TODO: Support customer time-zore.
#[derive(Debug, Default, Copy, Clone)]
/// Cycle plan task builder.
pub struct TaskBuilder<'a> {
    /// Repeat type.
    frequency: FrequencyUnify<'a>,

    /// Task_id should unique.
    task_id: u64,

    /// Maximum execution time (optional).
    /// it can be use to deadline (excution-time + maximum_running_time).
    maximum_running_time: Option<u64>,

    /// Maximum parallel runnable num (optional).
    maximum_parallel_runnable_num: Option<u64>,

    /// If it is built by set_frequency_by_candy, set the tag separately.
    build_by_candy_str: bool,

    /// Time zone for cron-expression iteration time.
    schedule_iterator_time_zone: ScheduleIteratorTimeZone,
}

#[derive(Debug, Clone, Default)]
/// Task runtime context.
pub struct TaskContext {
    /// The id of Task.
    pub task_id: u64,
    /// The id of the task running instance.
    pub record_id: i64,
    /// Hook functions that may be used in the future.
    pub then_fn: Option<fn()>,
    /// Async-Runtime Kind
    pub runtime_kind: RuntimeKind,
    /// Event Sender for Timer Wheel Core.
    pub(crate) timer_event_sender: Option<TimerEventSender>,
}

impl TaskContext {
    #[inline(always)]
    /// Get the id of task.
    pub fn task_id(&mut self, task_id: u64) -> &mut Self {
        self.task_id = task_id;
        self
    }

    #[inline(always)]
    /// Get the id of the task running instance.
    pub fn record_id(&mut self, record_id: i64) -> &mut Self {
        self.record_id = record_id;
        self
    }

    #[inline(always)]
    pub(crate) fn timer_event_sender(&mut self, timer_event_sender: TimerEventSender) -> &mut Self {
        self.timer_event_sender = Some(timer_event_sender);
        self
    }

    #[inline(always)]
    /// Get hook functions that may be used in the future.
    pub fn then_fn(&mut self, then_fn: fn()) -> &mut Self {
        self.then_fn = Some(then_fn);
        self
    }

    #[inline(always)]
    pub(crate) fn runtime_kind(&mut self, runtime_kind: RuntimeKind) -> &mut Self {
        self.runtime_kind = runtime_kind;
        self
    }

    /// Send a task-Finish signal to EventHandle.
    pub async fn finish_task(self, finish_output: Option<FinishOutput>) {
        if let Some(timer_event_sender) = self.timer_event_sender {
            timer_event_sender
                .send(TimerEvent::FinishTask(FinishTaskBody {
                    task_id: self.task_id,
                    record_id: self.record_id,
                    finish_time: timestamp(),
                    finish_output,
                }))
                .await
                .unwrap_or_else(|e| error!("{}", e));
        }
    }
}

//TODO:Future tasks will support single execution (not multiple executions in the same time frame).
type SafeBoxFn = Box<dyn Fn(TaskContext) -> Box<dyn DelayTaskHandler> + 'static + Send + Sync>;
type SafeBoxRoutine = Box<
    dyn Routine<TokioHandle = TokioJoinHandle<()>, SmolHandle = SmolJoinHandler<()>>
        + 'static
        + Send,
>;

pub(crate) struct SafeStructBoxedFn(pub(crate) SafeBoxFn);
impl fmt::Debug for SafeStructBoxedFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <&Self as Pointer>::fmt(&self, f)
    }
}

pub(crate) struct SafeStructBoxRoutine(pub(crate) SafeBoxRoutine);
impl fmt::Debug for SafeStructBoxRoutine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <&Self as Pointer>::fmt(&self, f)
    }
}
// Internal closures, once created
// Will not be changed (read-only access), so `Sync` can be implemented manually
unsafe impl Sync for SafeStructBoxRoutine {}
unsafe impl Sync for SafeStructBoxedFn {}

// For Async Task
#[derive(Debug, Clone)]
struct AsyncFn<F: Fn() -> U + Send + 'static, U: Future + Send + 'static>(F);

// For Sync Task
#[derive(Debug, Clone)]
struct SyncFn<F: Fn() + Send + 'static + Clone>(F);

// Routine abstractions performed during task execution.
pub(crate) trait Routine {
    type TokioHandle;
    type SmolHandle;
    fn spawn_by_tokio(&self, task_context: TaskContext) -> Self::TokioHandle;
    fn spawn_by_smol(&self, task_context: TaskContext) -> Self::SmolHandle;
}

impl<F: Fn() -> U + 'static + Send, U: Future + 'static + Send> Routine for AsyncFn<F, U> {
    type TokioHandle = TokioJoinHandle<()>;
    type SmolHandle = SmolJoinHandler<()>;

    #[inline(always)]
    fn spawn_by_tokio(&self, task_context: TaskContext) -> Self::TokioHandle {
        let user_future = self.0();

        async_spawn_by_tokio({
            let task_id = task_context.task_id;
            let record_id = task_context.record_id;
            async {
                user_future.await;
                task_context.finish_task(None).await;
            }
            .instrument(info_span!(
                "async_spawn_by_tokio: routine-exec",
                task_id,
                record_id
            ))
        })
    }

    #[inline(always)]
    fn spawn_by_smol(&self, task_context: TaskContext) -> Self::SmolHandle {
        let user_future = self.0();

        async_spawn_by_smol({
            let task_id = task_context.task_id;
            let record_id = task_context.record_id;
            async {
                user_future.await;
                task_context.finish_task(None).await;
            }
            .instrument(info_span!(
                "async_spawn_by_smol: routine-exec",
                task_id,
                record_id
            ))
        })
    }
}

// fn demonstrate_event_handle(){
// within EventHandle::add_task
// let body == if instance_kind == tokio { move || routine.spawn_by_tokio() }
// else instance_kind == smol { move || routine.spawn_by_smol() }
// Or
// let body == if instance_kind == tokio { Routine::spawn_by_tokio }
// else instance_kind == smol { Routine::spawn_by_smol }
// within timer-core body()
// }

impl<F: Fn() + 'static + Send + Clone> Routine for SyncFn<F> {
    type TokioHandle = TokioJoinHandle<()>;
    type SmolHandle = SmolJoinHandler<()>;

    #[inline(always)]
    fn spawn_by_tokio(&self, task_context: TaskContext) -> Self::TokioHandle {
        let fn_handle = unblock_spawn_by_tokio(self.0.clone());

        let task_id = task_context.task_id;
        let record_id = task_context.record_id;

        async_spawn_by_tokio({
            async {
                if let Err(e) = fn_handle.await {
                    error!("{}", e);
                }
                task_context.finish_task(None).await;
            }
            .instrument(info_span!(
                "async_spawn_by_smol: routine-exec",
                task_id,
                record_id
            ))
        })
    }

    #[inline(always)]
    fn spawn_by_smol(&self, task_context: TaskContext) -> Self::SmolHandle {
        let fn_handle = unblock_spawn_by_smol(self.0.clone());

        let task_id = task_context.task_id;
        let record_id = task_context.record_id;

        async_spawn_by_smol({
            async {
                fn_handle.await;
                task_context.finish_task(None).await;
            }
            .instrument(info_span!(
                "async_spawn_by_smol: routine-exec",
                task_id,
                record_id
            ))
        })
    }
}

#[derive(Debug)]
/// Periodic Task Structures.
pub struct Task {
    /// Unique task-id.
    pub task_id: u64,
    /// Routine is the soul of the task, including the execution instructions of the task.
    pub(crate) routine: SafeStructBoxRoutine,
    /// Iter of frequencies and executive clocks.
    frequency: FrequencyInner,
    /// Maximum execution time (optional).
    maximum_running_time: Option<u64>,
    /// Loop the line and check how many more clock cycles it will take to execute it.
    cylinder_line: u64,
    /// Validity.
    /// Any `Task` can set `valid` for that stop.
    valid: bool,
    /// Maximum parallel runnable num (optional).
    pub(crate) maximum_parallel_runnable_num: Option<u64>,
}

impl<'a> TaskBuilder<'a> {
    /// Set task Frequency.
    /// This api will be deprecated in the future, please use `set_frequency_once_*` | `set_frequency_count_down_*` | `set_frequency_repeated_*` etc.
    #[deprecated]
    #[inline(always)]
    pub fn set_frequency(&mut self, frequency: Frequency<'a>) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencyCronStr(frequency);
        self
    }

    /// Set task Frequency by customized CandyCronStr.
    /// In order to build a high-performance,
    /// highly reusable `TaskBuilder` that maintains the Copy feature .
    ///
    /// when supporting building from CandyCronStr ,
    /// here actively leaks memory for create a str-slice (because str-slice support Copy, String does not)
    ///
    /// We need to call `free` manually before `TaskBuilder` drop or before we leave the scope.
    ///
    /// Explain:
    /// Explicitly implementing both `Drop` and `Copy` trait on a type is currently
    /// disallowed.
    ///
    /// This feature can make some sense in theory, but the current
    /// implementation is incorrect and can lead to memory unsafety (see
    /// (issue #20126), so it has been disabled for now.

    /// This api will be deprecated in the future, please use `set_frequency_*_by_candy` etc.
    #[deprecated]
    #[inline(always)]
    pub fn set_frequency_by_candy<T: Into<CandyCronStr>>(
        &mut self,
        frequency: CandyFrequency<T>,
    ) -> &mut Self {
        self.build_by_candy_str = true;

        let frequency = match frequency {
            CandyFrequency::Once(candy_cron_middle_str) => {
                Frequency::Once(Box::leak(candy_cron_middle_str.into().0.into_boxed_str()))
            }
            CandyFrequency::Repeated(candy_cron_middle_str) => {
                Frequency::Repeated(Box::leak(candy_cron_middle_str.into().0.into_boxed_str()))
            }
            CandyFrequency::CountDown(exec_count, candy_cron_middle_str) => Frequency::CountDown(
                exec_count as u64,
                Box::leak(candy_cron_middle_str.into().0.into_boxed_str()),
            ),
        };

        self.frequency = FrequencyUnify::FrequencyCronStr(frequency);
        self
    }

    /// Set task-id.
    #[inline(always)]
    pub fn set_task_id(&mut self, task_id: u64) -> &mut Self {
        self.task_id = task_id;
        self
    }

    /// Set maximum execution time (optional).
    #[inline(always)]
    pub fn set_maximum_running_time(&mut self, maximum_running_time: u64) -> &mut Self {
        self.maximum_running_time = Some(maximum_running_time);
        self
    }

    /// Set a task with the maximum number of parallel runs (optional).
    #[inline(always)]
    pub fn set_maximum_parallel_runnable_num(
        &mut self,
        maximum_parallel_runnable_num: u64,
    ) -> &mut Self {
        self.maximum_parallel_runnable_num = Some(maximum_parallel_runnable_num);
        self
    }

    /// Set time zone for cron-expression iteration time.
    #[inline(always)]
    pub fn set_schedule_iterator_time_zone(
        &mut self,
        schedule_iterator_time_zone: ScheduleIteratorTimeZone,
    ) -> &mut Self {
        self.schedule_iterator_time_zone = schedule_iterator_time_zone;
        self
    }

    /// Spawn a task with async-routine.
    pub fn spawn_async_routine<
        F: Fn() -> U + 'static + Send,
        U: std::future::Future + 'static + Send,
    >(
        self,
        routine: F,
    ) -> Result<Task, TaskError> {
        let frequency_inner = (self.frequency, self.schedule_iterator_time_zone).try_into()?;

        Ok(Task {
            task_id: self.task_id,
            routine: SafeStructBoxRoutine(Box::new(AsyncFn(routine))),
            frequency: frequency_inner,
            maximum_running_time: self.maximum_running_time,
            cylinder_line: 0,
            valid: true,
            maximum_parallel_runnable_num: self.maximum_parallel_runnable_num,
        })
    }

    /// Spawn a task with sync-routine.
    pub fn spawn_routine<F: Fn() + 'static + Send + Clone>(
        self,
        routine: F,
    ) -> Result<Task, TaskError> {
        let frequency_inner = (self.frequency, self.schedule_iterator_time_zone).try_into()?;

        Ok(Task {
            task_id: self.task_id,
            routine: SafeStructBoxRoutine(Box::new(SyncFn(routine))),
            frequency: frequency_inner,
            maximum_running_time: self.maximum_running_time,
            cylinder_line: 0,
            valid: true,
            maximum_parallel_runnable_num: self.maximum_parallel_runnable_num,
        })
    }

    /// If we call set_frequency_by_candy explicitly and generate TaskBuilder,
    /// We need to call `free` manually before `TaskBuilder` drop or before we leave the scope.
    ///
    /// Explain:
    /// Explicitly implementing both `Drop` and `Copy` trait on a type is currently
    /// disallowed. This feature can make some sense in theory, but the current
    /// implementation is incorrect and can lead to memory unsafety (see
    /// (issue #20126), so it has been disabled for now.

    /// So I can't go through Drop and handle these automatically.
    pub fn free(&mut self) {
        if self.build_by_candy_str {
            let s = match self.frequency {
                FrequencyUnify::FrequencyCronStr(Frequency::Once(s)) => s,
                FrequencyUnify::FrequencyCronStr(Frequency::Repeated(s)) => s,
                FrequencyUnify::FrequencyCronStr(Frequency::CountDown(_, s)) => s,
                _ => return,
            };

            unsafe {
                Box::from_raw(std::mem::transmute::<&str, *mut str>(s));
            }
        }
    }
}

impl<'a> TaskBuilder<'a> {
    /// Task execution frequency: execute only once, set by cron expression.
    #[inline(always)]
    pub fn set_frequency_once_by_cron_str(&mut self, cron_str: &'a str) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencyCronStr(FrequencyCronStr::Once(cron_str));
        self
    }

    /// Task execution frequency: countdown execution, set by cron expression.
    #[inline(always)]
    pub fn set_frequency_repeated_by_cron_str(&mut self, cron_str: &'a str) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencyCronStr(FrequencyCronStr::Repeated(cron_str));
        self
    }

    /// Task execution frequency: execute repeatedly, set by cron expression.
    #[inline(always)]
    pub fn set_frequency_count_down_by_cron_str(
        &mut self,
        cron_str: &'a str,
        count_down: u64,
    ) -> &mut Self {
        self.frequency =
            FrequencyUnify::FrequencyCronStr(FrequencyCronStr::CountDown(count_down, cron_str));
        self
    }

    /// Task execution frequency: execute only once, set by seconds num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    #[inline(always)]
    pub fn set_frequency_once_by_seconds(&mut self, seconds: u64) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencySeconds(FrequencySeconds::Once(seconds));
        self
    }

    /// Task execution frequency: countdown execution, set by seconds num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    #[inline(always)]
    pub fn set_frequency_repeated_by_seconds(&mut self, seconds: u64) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencySeconds(FrequencySeconds::Repeated(seconds));
        self
    }

    /// Task execution frequency: execute repeatedly, set by seconds num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    #[inline(always)]
    pub fn set_frequency_count_down_by_seconds(
        &mut self,
        seconds: u64,
        count_down: u64,
    ) -> &mut Self {
        self.frequency =
            FrequencyUnify::FrequencySeconds(FrequencySeconds::CountDown(count_down, seconds));
        self
    }

    /// Task execution frequency: execute only once, set by minutes num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    pub fn set_frequency_once_by_minutes(&mut self, minutes: u64) -> &mut Self {
        self.frequency =
            FrequencyUnify::FrequencySeconds(FrequencySeconds::Once(ONE_MINUTE * minutes));
        self
    }

    /// Task execution frequency: countdown execution, set by minutes num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    #[inline(always)]
    pub fn set_frequency_repeated_by_minutes(&mut self, minutes: u64) -> &mut Self {
        self.frequency =
            FrequencyUnify::FrequencySeconds(FrequencySeconds::Repeated(ONE_MINUTE * minutes));
        self
    }

    /// Task execution frequency: execute repeatedly, set by minutes num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    #[inline(always)]
    pub fn set_frequency_count_down_by_minutes(
        &mut self,
        minutes: u64,
        count_down: u64,
    ) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencySeconds(FrequencySeconds::CountDown(
            count_down,
            ONE_MINUTE * minutes,
        ));
        self
    }

    /// Task execution frequency: execute only once, set by hours num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    pub fn set_frequency_once_by_hours(&mut self, hours: u64) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencySeconds(FrequencySeconds::Once(ONE_HOUR * hours));
        self
    }

    /// Task execution frequency: execute repeatedly, set by hours num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    #[inline(always)]
    pub fn set_frequency_repeated_by_hours(&mut self, hours: u64) -> &mut Self {
        self.frequency =
            FrequencyUnify::FrequencySeconds(FrequencySeconds::Repeated(ONE_HOUR * hours));
        self
    }

    /// Task execution frequency: countdown execution, set by hours num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    #[inline(always)]
    pub fn set_frequency_count_down_by_hours(&mut self, hours: u64, count_down: u64) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencySeconds(FrequencySeconds::CountDown(
            count_down,
            ONE_HOUR * hours,
        ));
        self
    }

    /// Task execution frequency: execute only once, set by days num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    pub fn set_frequency_once_by_days(&mut self, days: u64) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencySeconds(FrequencySeconds::Once(ONE_DAY * days));
        self
    }

    /// Task execution frequency: execute repeatedly, set by days num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    #[inline(always)]
    pub fn set_frequency_repeated_by_days(&mut self, days: u64) -> &mut Self {
        self.frequency =
            FrequencyUnify::FrequencySeconds(FrequencySeconds::Repeated(ONE_DAY * days));
        self
    }

    /// Task execution frequency: countdown execution, set by days num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    #[inline(always)]
    pub fn set_frequency_count_down_by_days(&mut self, days: u64, count_down: u64) -> &mut Self {
        self.frequency = FrequencyUnify::FrequencySeconds(FrequencySeconds::CountDown(
            count_down,
            ONE_DAY * days,
        ));
        self
    }

    /// Task execution frequency: execute only once, set by timestamp-seconds num.
    ///
    /// Make sure time is greater than 1 seconds, otherwise undefined behavior will be triggered.

    pub fn set_frequency_once_by_timestamp_seconds(&mut self, timestamp_seconds: u64) -> &mut Self {
        let duration = timestamp_seconds
            .checked_sub(timestamp())
            .unwrap_or(ONE_SECOND);

        self.frequency = FrequencyUnify::FrequencySeconds(FrequencySeconds::Once(duration));
        self
    }
}
impl Task {
    // swap slot loction ,do this
    // down_count_and_set_vaild,will return new vaild status.
    #[inline(always)]
    pub(crate) fn down_count_and_set_vaild(&mut self) -> bool {
        self.down_count();
        self.set_valid_by_count_down();
        self.is_valid()
    }

    // down_exec_count
    #[inline(always)]
    fn down_count(&mut self) {
        self.frequency.down_count();
    }

    // set_valid_by_count_down
    #[inline(always)]
    fn set_valid_by_count_down(&mut self) {
        self.valid = !self.frequency.is_down_over();
    }

    /// After task initialization or handled
    /// Redefine `cylinder_line`.
    #[inline(always)]
    pub(crate) fn set_cylinder_line(&mut self, cylinder_line: u64) {
        self.cylinder_line = cylinder_line;
    }

    #[inline(always)]
    /// Get the maximum running time of the task.
    pub fn get_maximum_running_time(&self, start_time: u64) -> Option<u64> {
        self.maximum_running_time.map(|t| t + start_time)
    }

    // single slot foreach do this.
    // sub_cylinder_line
    #[inline(always)]
    pub(crate) fn sub_cylinder_line(&mut self) {
        self.cylinder_line -= 1;
    }

    #[inline(always)]
    pub(crate) fn clear_cylinder_line(&mut self) {
        self.cylinder_line = 0;
    }

    #[inline(always)]
    /// check if task has arrived.
    pub fn check_arrived(&mut self) -> bool {
        trace!("check self: {:?}", self);

        if self.cylinder_line == 0 {
            return self.is_can_running();
        }

        self.sub_cylinder_line();
        false
    }

    /// check if task has already.
    #[inline(always)]
    pub fn is_already(&self) -> bool {
        self.cylinder_line == 0
    }

    /// check if task has runnable status.
    #[inline(always)]
    pub fn is_can_running(&self) -> bool {
        if self.is_valid() {
            return self.is_already();
        }
        false
    }

    /// check if task has valid status.
    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// get_next_exec_timestamp
    #[inline(always)]
    pub fn get_next_exec_timestamp(&mut self) -> Option<u64> {
        self.frequency.next_alarm_timestamp().map(|i| i as u64)
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]

    use super::{Task, TaskBuilder};
    use crate::prelude::*;
    use anyhow::Result as AnyResult;
    use rand::prelude::*;
    use std::iter::Iterator;

    #[test]
    fn test_task_valid() -> AnyResult<()> {
        let mut task_builder = TaskBuilder::default();

        // The third run returns to an invalid state.
        task_builder.set_frequency_count_down_by_seconds(1, 3);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        assert!(task.down_count_and_set_vaild());
        assert!(task.down_count_and_set_vaild());
        assert!(!task.down_count_and_set_vaild());

        task_builder.set_frequency_count_down_by_cron_str("* * * * * * *", 3);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        assert!(task.down_count_and_set_vaild());
        assert!(task.down_count_and_set_vaild());
        assert!(!task.down_count_and_set_vaild());

        Ok(())
    }

    #[test]
    fn test_get_next_exec_timestamp_seconds() -> AnyResult<()> {
        let mut rng = rand::thread_rng();
        let init_seconds: u64 = rng.gen_range(1..100_00_00);
        let mut task_builder = TaskBuilder::default();

        task_builder.set_frequency_count_down_by_seconds(init_seconds, 3);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        (1..100)
            .map(|i| {
                debug_assert_eq!(
                    task.get_next_exec_timestamp().unwrap(),
                    timestamp() + (init_seconds * i)
                );
            })
            .for_each(drop);

        task_builder.set_frequency_count_down_by_cron_str("* * * * * * *", 100);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        (1..100)
            .map(|_| {
                assert!(task.down_count_and_set_vaild());
            })
            .for_each(drop);

        assert!(!task.down_count_and_set_vaild());

        Ok(())
    }

    #[test]
    fn test_get_next_exec_timestamp_minutes() -> AnyResult<()> {
        let mut rng = rand::thread_rng();
        let init_minutes: u64 = rng.gen_range(1..100_00_00);
        let mut task_builder = TaskBuilder::default();

        task_builder.set_frequency_repeated_by_minutes(init_minutes);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        (1..100)
            .map(|i| {
                debug_assert_eq!(
                    task.get_next_exec_timestamp().unwrap(),
                    timestamp() + (init_minutes * i * ONE_MINUTE)
                );
            })
            .for_each(drop);

        Ok(())
    }

    #[test]
    fn test_get_next_exec_timestamp_hours() -> AnyResult<()> {
        let mut rng = rand::thread_rng();
        let init_hours: u64 = rng.gen_range(1..100_00_00);
        let mut task_builder = TaskBuilder::default();

        task_builder.set_frequency_repeated_by_hours(init_hours);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        (1..100)
            .map(|i| {
                debug_assert_eq!(
                    task.get_next_exec_timestamp().unwrap(),
                    timestamp() + (init_hours * i * ONE_HOUR)
                );
            })
            .for_each(drop);

        Ok(())
    }

    #[test]
    fn test_get_next_exec_timestamp_days() -> AnyResult<()> {
        let mut rng = rand::thread_rng();
        let init_days: u64 = rng.gen_range(1..100_00_00);
        let mut task_builder = TaskBuilder::default();

        task_builder.set_frequency_repeated_by_days(init_days);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        (1..100)
            .map(|i| {
                debug_assert_eq!(
                    task.get_next_exec_timestamp().unwrap(),
                    timestamp() + (init_days * i * ONE_DAY)
                );
            })
            .for_each(drop);

        Ok(())
    }

    #[test]
    fn test_count_down() -> AnyResult<()> {
        let mut task_builder = TaskBuilder::default();

        // The third run returns to an invalid state.
        task_builder.set_frequency_count_down_by_seconds(1, 3);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        assert!(task.down_count_and_set_vaild());
        assert!(task.down_count_and_set_vaild());
        assert!(!task.down_count_and_set_vaild());

        task_builder.set_frequency_count_down_by_cron_str("* * * * * * *", 3);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        assert!(task.down_count_and_set_vaild());
        assert!(task.down_count_and_set_vaild());
        assert!(!task.down_count_and_set_vaild());

        task_builder.set_frequency_once_by_seconds(10);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;
        assert!(!task.down_count_and_set_vaild());

        task_builder.set_frequency_count_down_by_hours(10, 10);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;
        (1i32..10i32)
            .map(|_| assert!(task.down_count_and_set_vaild()))
            .for_each(drop);
        assert!(!task.down_count_and_set_vaild());

        Ok(())
    }

    #[test]
    fn test_is_can_running() -> AnyResult<()> {
        let mut task_builder = TaskBuilder::default();

        // The third run returns to an invalid state.
        task_builder.set_frequency_count_down_by_cron_str("* * * * * * *", 3);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        assert!(task.is_can_running());

        task.set_cylinder_line(1);
        assert!(!task.is_can_running());

        assert!(!task.check_arrived());
        assert!(task.is_can_running());

        // set_frequency_count_down_by_seconds.
        task_builder.set_frequency_count_down_by_seconds(1, 1);
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        assert!(task.is_can_running());

        task.set_cylinder_line(1);
        assert!(!task.is_can_running());

        assert!(!task.check_arrived());
        assert!(task.is_can_running());

        Ok(())
    }

    #[test]
    fn test_candy_cron() -> AnyResult<()> {
        use super::{CandyCron, CandyFrequency, Task, TaskBuilder};
        let mut task_builder = TaskBuilder::default();

        // The third run returns to an invalid state.
        task_builder.set_frequency_by_candy(CandyFrequency::CountDown(5, CandyCron::Minutely));
        let mut task: Task = task_builder.spawn_async_routine(|| async {})?;

        assert!(task.is_can_running());

        task.set_cylinder_line(1);
        assert!(!task.is_can_running());

        assert!(!task.check_arrived());
        assert!(task.is_can_running());

        Ok(())
    }

    #[test]
    fn test_analyze_cron_expression() -> AnyResult<()> {
        use super::{DelayTimerScheduleIteratorOwned, ScheduleIteratorTimeZone};
        use std::thread::sleep;
        use std::time::Duration;

        let mut schedule_iterator_first = DelayTimerScheduleIteratorOwned::analyze_cron_expression(
            ScheduleIteratorTimeZone::Utc,
            "0/3 * * * * * *",
        )?;

        sleep(Duration::from_secs(5));

        let mut schedule_iterator_second =
            DelayTimerScheduleIteratorOwned::analyze_cron_expression(
                ScheduleIteratorTimeZone::Utc,
                "0/3 * * * * * *",
            )?;

        // Two different starting values should not be the same.
        assert_ne!(
            schedule_iterator_first.next(),
            schedule_iterator_second.next()
        );

        Ok(())
    }
}
