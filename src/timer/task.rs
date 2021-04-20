//! Task
//! It is a basic periodic task execution unit.
use super::runtime_trace::task_handle::DelayTaskHandler;
use crate::prelude::*;

use std::cell::RefCell;
use std::fmt;
use std::fmt::Pointer;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::thread::AccessError;

use cron_clock::{Schedule, ScheduleIteratorOwned, Utc};
use lru::LruCache;

//TODO: Add doc.
thread_local!(static CRON_EXPRESSION_CACHE: RefCell<LruCache<ScheduleIteratorTimeZoneQuery, DelayTimerScheduleIteratorOwned>> = RefCell::new(LruCache::new(256)));

// TaskMark is used to maintain the status of running tasks.
#[derive(Default, Debug)]
pub(crate) struct TaskMark {
    // The id of task.
    pub(crate) task_id: u64,
    // The wheel slot where the task is located.
    slot_mark: u64,
    // Number of tasks running in parallel.
    parallel_runable_num: u64,
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
    pub(crate) fn get_parallel_runable_num(&self) -> u64 {
        self.parallel_runable_num
    }

    #[inline(always)]
    pub(crate) fn set_parallel_runable_num(&mut self, parallel_runable_num: u64) -> &mut Self {
        self.parallel_runable_num = parallel_runable_num;
        self
    }

    #[inline(always)]
    pub(crate) fn inc_parallel_runable_num(&mut self) {
        self.parallel_runable_num += 1;
    }

    #[inline(always)]
    pub(crate) fn dec_parallel_runable_num(&mut self) {
        self.parallel_runable_num = self.parallel_runable_num.checked_sub(1).unwrap_or_default();
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
    ) -> Option<Instance> {
        let task_instances_chain_maintainer = self.get_task_instances_chain_maintainer()?;

        let index = task_instances_chain_maintainer
            .inner_list
            .iter()
            .position(|d| d.get_record_id() == record_id)?;

        let mut has_remove_instance_list =
            task_instances_chain_maintainer.inner_list.split_off(index);
        let remove_instance = has_remove_instance_list.pop_front();
        task_instances_chain_maintainer
            .inner_list
            .append(&mut has_remove_instance_list);

        if let Some(i) = remove_instance.as_ref() {
            i.notify_cancel_finish(state)
        }
        remove_instance
    }
}

#[derive(Debug, Copy, Clone)]
/// Enumerated values of repeating types.
pub enum Frequency<'a> {
    /// Repeat once.
    Once(&'a str),
    /// Repeat ad infinitum.
    Repeated(&'a str),
    /// Type of countdown.
    CountDown(u32, &'a str),
}
/// Iterator for task internal control of execution time.
#[derive(Debug, Clone)]
pub(crate) enum FrequencyInner {
    ///Unlimited repetition types.
    Repeated(DelayTimerScheduleIteratorOwned),
    ///Type of countdown.
    CountDown(u32, DelayTimerScheduleIteratorOwned),
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
    ) -> DelayTimerScheduleIteratorOwned {
        match time_zone {
            ScheduleIteratorTimeZone::Utc => DelayTimerScheduleIteratorOwned::Utc(
                Schedule::from_str(cron_expression)
                    .unwrap()
                    .upcoming_owned(Utc),
            ),
            ScheduleIteratorTimeZone::Local => DelayTimerScheduleIteratorOwned::Local(
                Schedule::from_str(cron_expression)
                    .unwrap()
                    .upcoming_owned(Local),
            ),
            ScheduleIteratorTimeZone::FixedOffset(fixed_offset) => {
                DelayTimerScheduleIteratorOwned::FixedOffset(
                    Schedule::from_str(cron_expression)
                        .unwrap()
                        .upcoming_owned(fixed_offset),
                )
            }
        }
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
    pub(crate) fn next(&mut self) -> i64 {
        match self {
            Self::Utc(ref mut iterator) => iterator.next().unwrap().timestamp(),
            Self::Local(ref mut iterator) => iterator.next().unwrap().timestamp(),
            Self::FixedOffset(ref mut iterator) => iterator.next().unwrap().timestamp(),
        }
    }

    // Analyze expressions, get cache.
    fn analyze_cron_expression(
        time_zone: ScheduleIteratorTimeZone,
        cron_expression: &str,
    ) -> Result<DelayTimerScheduleIteratorOwned, AccessError> {
        let indiscriminate_expression = cron_expression.trim_matches(' ').to_owned();
        let schedule_iterator_time_zone_query: ScheduleIteratorTimeZoneQuery =
            ScheduleIteratorTimeZoneQuery {
                cron_expression: indiscriminate_expression,
                time_zone,
            };

        CRON_EXPRESSION_CACHE.try_with(|expression_cache| {
            let mut lru_cache = expression_cache.borrow_mut();
            if let Some(schedule_iterator) = lru_cache.get(&schedule_iterator_time_zone_query) {
                let mut schedule_iterator_copy = schedule_iterator.clone();

                // Reset the internal base time to avoid expiration time during internal iterations.
                schedule_iterator_copy.refresh_previous_datetime(time_zone);

                return schedule_iterator_copy;
            }
            let task_schedule =
                DelayTimerScheduleIteratorOwned::new(schedule_iterator_time_zone_query.clone());
            lru_cache.put(schedule_iterator_time_zone_query, task_schedule.clone());
            task_schedule
        })
    }
}

impl FrequencyInner {
    //How many times the acquisition needs to be performed.
    #[allow(dead_code)]
    fn residual_time(&self) -> u32 {
        match self {
            FrequencyInner::Repeated(_) => u32::MAX,
            FrequencyInner::CountDown(ref time, _) => *time,
        }
    }

    fn next_alarm_timestamp(&mut self) -> i64 {
        //TODO:handle error
        match self {
            FrequencyInner::CountDown(_, ref mut clock) => clock.next(),
            FrequencyInner::Repeated(ref mut clock) => clock.next(),
        }
    }

    #[warn(unused_parens)]
    fn down_count(&mut self) {
        match self {
            FrequencyInner::CountDown(ref mut exec_count, _) => {
                *exec_count -= 1u32;
            }
            FrequencyInner::Repeated(_) => {}
        };
    }

    fn is_down_over(&mut self) -> bool {
        !matches!(self, FrequencyInner::CountDown(0, _))
    }
}

//TODO: Support customer time-zore.
#[derive(Debug, Default, Copy, Clone)]
/// Cycle plan task builder.
pub struct TaskBuilder<'a> {
    /// Repeat type.
    frequency: Option<Frequency<'a>>,

    /// Task_id should unique.
    task_id: u64,

    /// Maximum execution time (optional).
    /// it can be use to deadline (excution-time + maximum_running_time).
    maximum_running_time: Option<u64>,

    /// Maximum parallel runable num (optional).
    maximun_parallel_runable_num: Option<u64>,

    /// If it is built by set_frequency_by_candy, set the tag separately.
    build_by_candy_str: bool,

    /// Time zone for cron-expression iteration time.
    schedule_iterator_time_zone: ScheduleIteratorTimeZone,
}

//TODO:Future tasks will support single execution (not multiple executions in the same time frame).
type SafeBoxFn = Box<dyn Fn(TaskContext) -> Box<dyn DelayTaskHandler> + 'static + Send + Sync>;

#[derive(Debug, Clone, Default)]
/// Task runtime context.
pub struct TaskContext {
    /// The id of Task.
    pub task_id: u64,
    /// The id of the task running instance.
    pub record_id: i64,
    /// Hook functions that may be used in the future.
    pub then_fn: Option<fn()>,
    /// Event Sender for Timer Wheel Core.
    pub(crate) timer_event_sender: Option<TimerEventSender>,
}

impl TaskContext {
    /// Get the id of task.
    pub fn task_id(&mut self, task_id: u64) -> &mut Self {
        self.task_id = task_id;
        self
    }

    /// Get the id of the task running instance.
    pub fn record_id(&mut self, record_id: i64) -> &mut Self {
        self.record_id = record_id;
        self
    }

    pub(crate) fn timer_event_sender(&mut self, timer_event_sender: TimerEventSender) -> &mut Self {
        self.timer_event_sender = Some(timer_event_sender);
        self
    }

    /// Get hook functions that may be used in the future.
    pub fn then_fn(&mut self, then_fn: fn()) -> &mut Self {
        self.then_fn = Some(then_fn);
        self
    }

    /// Send a task-Finish signal to EventHandle.
    pub async fn finishe_task(self) {
        if let Some(timer_event_sender) = self.timer_event_sender {
            timer_event_sender
                .send(TimerEvent::FinishTask(
                    self.task_id,
                    self.record_id,
                    get_timestamp(),
                ))
                .await
                .unwrap();
        }
    }
}

pub(crate) struct SafeStructBoxedFn(pub(crate) SafeBoxFn);
impl fmt::Debug for SafeStructBoxedFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <&Self as Pointer>::fmt(&self, f).unwrap();
        Ok(())
    }
}

#[derive(Debug)]
/// Periodic Task Structures.
pub struct Task {
    /// Unique task-id.
    pub task_id: u64,
    /// Iter of frequencies and executive clocks.
    frequency: FrequencyInner,
    /// A Fn in box it can be run and return delayTaskHandler.
    pub(crate) body: SafeStructBoxedFn,
    /// Maximum execution time (optional).
    maximum_running_time: Option<u64>,
    /// Loop the line and check how many more clock cycles it will take to execute it.
    cylinder_line: u64,
    /// Validity.
    /// Any `Task` can set `valid` for that stop.
    valid: bool,
    /// Maximum parallel runable num (optional).
    pub(crate) maximun_parallel_runable_num: Option<u64>,
}

//bak type BoxFn
// type BoxFn = Box<dyn Fn(i32) -> Pin<Box<dyn Future<Output = i32>>>>;

enum RepeatType {
    Num(u32),
    Always,
}

impl<'a> TaskBuilder<'a> {
    /// Set task Frequency.
    #[inline(always)]
    pub fn set_frequency(&mut self, frequency: Frequency<'a>) -> &mut Self {
        self.frequency = Some(frequency);
        self
    }

    /// Set task Frequency by customized CandyCronStr.
    /// In order to build a high-performance,
    /// highly reusable `TaskBuilder` that maintains the Copy feature .
    /// when supporting building from CandyCronStr ,
    /// here actively leaks memory for create a str-slice (because str-slice support Copy, String does not)
    /// We need to call `free` manually before `TaskBuilder` drop or before we leave the scope.
    ///
    /// Explain:
    /// Explicitly implementing both `Drop` and `Copy` trait on a type is currently
    /// disallowed. This feature can make some sense in theory, but the current
    /// implementation is incorrect and can lead to memory unsafety (see
    /// [issue #20126][iss20126]), so it has been disabled for now.

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
                exec_count,
                Box::leak(candy_cron_middle_str.into().0.into_boxed_str()),
            ),
        };

        self.frequency = Some(frequency);
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
    pub fn set_maximun_parallel_runable_num(
        &mut self,
        maximun_parallel_runable_num: u64,
    ) -> &mut Self {
        self.maximun_parallel_runable_num = Some(maximun_parallel_runable_num);
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

    /// Spawn a task.
    pub fn spawn<F>(self, body: F) -> Result<Task, AccessError>
    where
        F: Fn(TaskContext) -> Box<dyn DelayTaskHandler> + 'static + Send + Sync,
    {
        let frequency_inner;

        // The user inputs are pattern matched for different repetition types.
        let (expression_str, repeat_type) = match self.frequency.unwrap() {
            Frequency::Once(expression_str) => (expression_str, RepeatType::Num(1)),
            Frequency::Repeated(expression_str) => (expression_str, RepeatType::Always),
            Frequency::CountDown(exec_count, expression_str) => {
                (expression_str, RepeatType::Num(exec_count))
            }
        };

        let taskschedule = DelayTimerScheduleIteratorOwned::analyze_cron_expression(
            self.schedule_iterator_time_zone,
            expression_str,
        )?;

        // Building TaskFrequencyInner patterns based on repetition types.
        frequency_inner = match repeat_type {
            RepeatType::Always => FrequencyInner::Repeated(taskschedule),
            RepeatType::Num(repeat_count) => FrequencyInner::CountDown(repeat_count, taskschedule),
        };

        let body = SafeStructBoxedFn(Box::new(body));

        Ok(Task {
            task_id: self.task_id,
            frequency: frequency_inner,
            body,
            maximum_running_time: self.maximum_running_time,
            cylinder_line: 0,
            valid: true,
            maximun_parallel_runable_num: self.maximun_parallel_runable_num,
        })
    }

    /// If we call set_frequency_by_candy explicitly and generate TaskBuilder,
    /// We need to call `free` manually before `TaskBuilder` drop or before we leave the scope.
    ///
    /// Explain:
    /// Explicitly implementing both `Drop` and `Copy` trait on a type is currently
    /// disallowed. This feature can make some sense in theory, but the current
    /// implementation is incorrect and can lead to memory unsafety (see
    /// [issue #20126][iss20126]), so it has been disabled for now.

    /// So I can't go through Drop and handle these automatically.
    pub fn free(&mut self) {
        if self.build_by_candy_str {
            if let Some(frequency) = self.frequency {
                let s = match frequency {
                    Frequency::Once(s) => s,
                    Frequency::Repeated(s) => s,
                    Frequency::CountDown(_, s) => s,
                };

                unsafe {
                    Box::from_raw(std::mem::transmute::<&str, *mut str>(s));
                }
            }
        }
    }
}

impl Task {
    // get SafeBoxFn of Task to call.
    #[inline(always)]
    pub(crate) fn get_body(&self) -> &SafeBoxFn {
        &(self.body).0
    }

    // swap slot loction ,do this
    // down_count_and_set_vaild,will return new vaild status.
    #[inline(always)]
    pub(crate) fn down_count_and_set_vaild(&mut self) -> bool {
        self.down_count();
        self.set_valid_by_count_down();
        self.is_valid()
    }

    //down_exec_count
    #[inline(always)]
    fn down_count(&mut self) {
        self.frequency.down_count();
    }

    //set_valid_by_count_down
    #[inline(always)]
    fn set_valid_by_count_down(&mut self) {
        self.valid = self.frequency.is_down_over();
    }

    #[inline(always)]
    pub(crate) fn set_cylinder_line(&mut self, cylinder_line: u64) {
        self.cylinder_line = cylinder_line;
    }

    #[inline(always)]
    /// Get the maximum running time of the task.
    pub fn get_maximum_running_time(&self, start_time: u64) -> Option<u64> {
        self.maximum_running_time.map(|t| t + start_time)
    }

    //single slot foreach do this.
    //sub_cylinder_line
    //return is can_running?
    #[inline(always)]
    pub(crate) fn sub_cylinder_line(&mut self) -> bool {
        self.cylinder_line -= 1;
        self.is_can_running()
    }

    #[inline(always)]
    /// check if task has arrived.
    pub fn check_arrived(&mut self) -> bool {
        if self.cylinder_line == 0 {
            return self.is_can_running();
        }
        self.sub_cylinder_line()
    }

    /// check if task has already.
    #[inline(always)]
    pub fn is_already(&self) -> bool {
        self.cylinder_line == 0
    }

    /// check if task has runable status.
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

    ///get_next_exec_timestamp
    #[inline(always)]
    pub fn get_next_exec_timestamp(&mut self) -> u64 {
        self.frequency.next_alarm_timestamp() as u64
    }
}

mod tests {

    #[test]
    fn test_task_valid() {
        use super::{Frequency, Task, TaskBuilder};
        use crate::utils::convenience::functions::create_default_delay_task_handler;
        let mut task_builder = TaskBuilder::default();

        //The third run returns to an invalid state.
        task_builder.set_frequency(Frequency::CountDown(3, "* * * * * * *"));
        let mut task: Task = task_builder
            .spawn(|_context| create_default_delay_task_handler())
            .unwrap();

        assert!(task.down_count_and_set_vaild());
        assert!(task.down_count_and_set_vaild());
        assert!(!task.down_count_and_set_vaild());
    }

    #[test]
    fn test_is_can_running() {
        use super::{Frequency, Task, TaskBuilder};
        use crate::utils::convenience::functions::create_default_delay_task_handler;
        let mut task_builder = TaskBuilder::default();

        //The third run returns to an invalid state.
        task_builder.set_frequency(Frequency::CountDown(3, "* * * * * * *"));
        let mut task: Task = task_builder
            .spawn(|_context| create_default_delay_task_handler())
            .unwrap();

        assert!(task.is_can_running());

        task.set_cylinder_line(1);
        assert!(!task.is_can_running());

        assert!(task.check_arrived());
    }

    // struct CandyCron

    #[test]
    fn test_candy_cron() {
        use super::{CandyCron, CandyFrequency, Task, TaskBuilder};
        use crate::utils::convenience::functions::create_default_delay_task_handler;
        let mut task_builder = TaskBuilder::default();

        //The third run returns to an invalid state.
        task_builder.set_frequency_by_candy(CandyFrequency::CountDown(5, CandyCron::Minutely));
        let mut task: Task = task_builder
            .spawn(|_context| create_default_delay_task_handler())
            .unwrap();

        assert!(task.is_can_running());

        task.set_cylinder_line(1);
        assert!(!task.is_can_running());

        assert!(task.check_arrived());
    }

    // TODO: should reimplement Clone for ScheduleIteratorOwned.
    #[test]
    fn test_analyze_cron_expression() {
        use super::{DelayTimerScheduleIteratorOwned, ScheduleIteratorTimeZone};
        use std::thread::sleep;
        use std::time::Duration;

        let mut schedule_iterator_first = DelayTimerScheduleIteratorOwned::analyze_cron_expression(
            ScheduleIteratorTimeZone::Utc,
            "0/6 * * * * * *",
        )
        .unwrap();

        sleep(Duration::from_secs(5));

        let mut schedule_iterator_second =
            DelayTimerScheduleIteratorOwned::analyze_cron_expression(
                ScheduleIteratorTimeZone::Utc,
                "0/6 * * * * * *",
            )
            .unwrap();

        // Two different starting values should not be the same.
        assert_ne!(
            schedule_iterator_first.next(),
            schedule_iterator_second.next()
        );
    }
}
