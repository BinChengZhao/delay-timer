use super::runtime_trace::task_handle::DelayTaskHandler;
use cron_clock::{Schedule, ScheduleIteratorOwned, Utc};
use lru::LruCache;
use std::str::FromStr;

//TODO: Add doc.
thread_local!(static CRON_EXPRESSION_CACHE: LruCache<&'static str, ScheduleIteratorOwned<Utc>> = LruCache::new(256));

//TaskMark is use to remove/stop the task.
#[derive(Default)]
pub(crate) struct TaskMark {
    pub(crate) task_id: u64,
    slot_mark: u64,
}

impl TaskMark {
    pub(crate) fn new(task_id: u64, slot_mark: u64) -> Self {
        TaskMark { task_id, slot_mark }
    }

    pub(crate) fn get_slot_mark(&self) -> u64 {
        self.slot_mark
    }

    pub(crate) fn set_slot_mark(&mut self, slot_mark: u64) {
        self.slot_mark = slot_mark;
    }
}

#[derive(Debug, Copy, Clone)]
///Enumerated values of repeating types.
pub enum Frequency<'a> {
    ///Repeat once.
    Once(&'a str),
    ///Repeat ad infinitum.
    Repeated(&'a str),
    ///Type of countdown.
    CountDown(u32, &'a str),
}
///Iterator for task internal control of execution time.
pub(crate) enum FrequencyInner {
    ///Unlimited repetition types.
    Repeated(ScheduleIteratorOwned<Utc>),
    ///Type of countdown.
    CountDown(u32, ScheduleIteratorOwned<Utc>),
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
            FrequencyInner::CountDown(_, ref mut clock) => clock.next().unwrap().timestamp(),
            FrequencyInner::Repeated(ref mut clock) => clock.next().unwrap().timestamp(),
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

#[derive(Debug, Default, Copy, Clone)]
///Cycle plan task builder.
pub struct TaskBuilder<'a> {
    ///Repeat type.
    frequency: Option<Frequency<'a>>,

    ///Task_id should unique.
    task_id: u64,

    ///Maximum execution time (optional).
    maximum_running_time: Option<u64>,
}

//TODO: 张老师建议参考 ruby 那些库，设计的很人性化.

//TODO:油条哥建议 cron 表达式的提供几个快捷enum， 这样IDE提示更方便。。
//太细节了。

//TODO:油条哥建议去除 api前的 set_，看起来更人性化。  但是有待权衡
//TASK 执行完了，支持找新的Slot
type SafeBoxFn = Box<dyn Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync>;
pub struct Task {
    ///Unique task-id.
    pub task_id: u64,
    ///Iter of frequencies and executive clocks.
    frequency: FrequencyInner,
    /// A Fn in box it can be run and return delayTaskHandler.
    pub(crate) body: SafeBoxFn,
    ///Maximum execution time (optional).
    maximum_running_time: Option<u64>,
    ///Loop the line and check how many more clock cycles it will take to execute it.
    cylinder_line: u64,
    ///Validity.
    valid: bool,
}

//bak type BoxFn
// type BoxFn = Box<dyn Fn(i32) -> Pin<Box<dyn Future<Output = i32>>>>;

enum RepeatType {
    Num(u32),
    Always,
}

impl<'a> TaskBuilder<'a> {
    ///Set task Frequency.
    #[inline(always)]
    pub fn set_frequency(&mut self, frequency: Frequency<'a>) -> &mut Self {
        self.frequency = Some(frequency);
        self
    }

    ///Set task-id.
    #[inline(always)]
    pub fn set_task_id(&mut self, task_id: u64) -> &mut Self {
        self.task_id = task_id;
        self
    }

    ///Set maximum execution time (optional).
    #[inline(always)]
    pub fn set_maximum_running_time(&mut self, maximum_running_time: u64) -> &mut Self {
        self.maximum_running_time = Some(maximum_running_time);
        self
    }

    ///Spawn a task.
    pub fn spawn<F>(self, body: F) -> Task
    where
        F: Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync,
    {
        let frequency_inner;

        //The user inputs are pattern matched for different repetition types.
        let (expression_str, repeat_type) = match self.frequency.unwrap() {
            Frequency::Once(expression_str) => (expression_str, RepeatType::Num(1)),
            Frequency::Repeated(expression_str) => (expression_str, RepeatType::Always),
            Frequency::CountDown(exec_count, expression_str) => {
                (expression_str, RepeatType::Num(exec_count))
            }
        };

        //From cron-expression-str build time-iter.
        let schedule = Schedule::from_str(expression_str).unwrap();
        let taskschedule = schedule.upcoming_owned(Utc);

        //Building TaskFrequencyInner patterns based on repetition types.
        frequency_inner = match repeat_type {
            RepeatType::Always => FrequencyInner::Repeated(taskschedule),
            RepeatType::Num(repeat_count) => FrequencyInner::CountDown(repeat_count, taskschedule),
        };

        Task::new(
            self.task_id,
            frequency_inner,
            Box::new(body),
            self.maximum_running_time,
        )
    }

    fn analyze_cron_expression(cron_expression: &str) ->ScheduleIteratorOwned<Utc> {

        let indiscriminate_expression = cron_expression.trim_matches(' ');
        //TODO: NOTICE cron-expression suger.
        //find cache result in `CRON_EXPRESSION_CACHE` by indiscriminate_expression.
        //From cron-expression-str build time-iter.
        let schedule = Schedule::from_str(cron_expression).unwrap();
        let taskschedule = schedule.upcoming_owned(Utc);
        todo!();
    }
}

impl Task {
    #[inline(always)]
    pub(crate) fn new(
        task_id: u64,
        frequency: FrequencyInner,
        body: SafeBoxFn,
        maximum_running_time: Option<u64>,
    ) -> Task {
        Task {
            task_id,
            frequency,
            body,
            maximum_running_time,
            cylinder_line: 0,
            valid: true,
        }
    }

    //swap slot loction ,do this
    //down_count_and_set_vaild,will return new vaild status.
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
