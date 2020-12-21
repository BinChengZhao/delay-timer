use super::runtime_trace::task_handle::DelayTaskHandler;
use crate::prelude::*;
use cron_clock::{Schedule, ScheduleIteratorOwned, Utc};
use lru::LruCache;
use std::cell::RefCell;
use std::fmt;
use std::fmt::Pointer;
use std::str::FromStr;
use std::thread::AccessError;

//TODO: Add doc.
thread_local!(static CRON_EXPRESSION_CACHE: RefCell<LruCache<String, ScheduleIteratorOwned<Utc>>> = RefCell::new(LruCache::new(256)));

//TaskMark is use to remove/stop the task.
#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct TaskMark {
    pub(crate) task_id: u64,
    slot_mark: u64,
    parallel_runable_num: u64,
}

impl TaskMark {
    pub(crate) fn new(task_id: u64, slot_mark: u64, parallel_runable_num: u64) -> Self {
        TaskMark {
            task_id,
            slot_mark,
            parallel_runable_num,
        }
    }

    pub(crate) fn get_slot_mark(&self) -> u64 {
        self.slot_mark
    }

    pub(crate) fn get_parallel_runable_num(&self) -> u64 {
        self.parallel_runable_num
    }

    pub(crate) fn set_slot_mark(&mut self, slot_mark: u64) {
        self.slot_mark = slot_mark;
    }

    pub(crate) fn inc_parallel_runable_num(&mut self) {
        self.parallel_runable_num = self.parallel_runable_num + 1;
    }

    pub(crate) fn dec_parallel_runable_num(&mut self) {
        self.parallel_runable_num = self.parallel_runable_num.checked_sub(1).unwrap_or_default();
    }
}
//TODO: Add CronCandy version.

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
#[derive(Debug, Clone)]
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

    ///Maximum parallel runable num (optional).
    maximun_parallel_runable_num: Option<u64>,
}

//TODO: 张老师建议参考 ruby 那些库，设计的很人性化.

//TASK 执行完了，支持找新的Slot
//TODO:Future tasks will support single execution (not multiple executions in the same time frame).
type SafeBoxFn = Box<dyn Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync>;

pub(crate) struct SafeStructBoxedFn(pub(crate) SafeBoxFn);
impl fmt::Debug for SafeStructBoxedFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <&Self as Pointer>::fmt(&self, f).unwrap();
        Ok(())
    }
}

#[derive(Debug)]
pub struct Task {
    ///Unique task-id.
    pub task_id: u64,
    ///Iter of frequencies and executive clocks.
    frequency: FrequencyInner,
    /// A Fn in box it can be run and return delayTaskHandler.
    pub(crate) body: SafeStructBoxedFn,
    ///Maximum execution time (optional).
    maximum_running_time: Option<u64>,
    ///Loop the line and check how many more clock cycles it will take to execute it.
    cylinder_line: u64,
    ///Validity.
    valid: bool,
    ///Maximum parallel runable num (optional).
    pub(crate) maximun_parallel_runable_num: u64,
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

    #[inline(always)]
    pub fn set_frequency_by_candy<T: Into<CandyCronStr>>(
        &mut self,
        frequency: CandyFrequency<T>,
    ) -> &mut Self {
        let frequency = match frequency {
            CandyFrequency::Once(candy_cron_middle_str) => {
                Frequency::Once(candy_cron_middle_str.into().0)
            }
            CandyFrequency::Repeated(candy_cron_middle_str) => {
                Frequency::Repeated(candy_cron_middle_str.into().0)
            }
            CandyFrequency::CountDown(exec_count, candy_cron_middle_str) => {
                Frequency::CountDown(exec_count, candy_cron_middle_str.into().0)
            }
        };

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
    pub fn spawn<F>(self, body: F) -> Result<Task, AccessError>
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

        let taskschedule = Self::analyze_cron_expression(expression_str)?;

        //Building TaskFrequencyInner patterns based on repetition types.
        frequency_inner = match repeat_type {
            RepeatType::Always => FrequencyInner::Repeated(taskschedule),
            RepeatType::Num(repeat_count) => FrequencyInner::CountDown(repeat_count, taskschedule),
        };

        Ok(Task::new(
            self.task_id,
            frequency_inner,
            Box::new(body),
            self.maximum_running_time,
            self.maximun_parallel_runable_num.unwrap(),
        ))
    }

    // Analyze expressions, get cache.
    fn analyze_cron_expression(
        cron_expression: &str,
    ) -> Result<ScheduleIteratorOwned<Utc>, AccessError> {
        let indiscriminate_expression = cron_expression.trim_matches(' ').to_owned();

        CRON_EXPRESSION_CACHE.try_with(|expression_cache| {
            let mut lru_cache = expression_cache.borrow_mut();
            if let Some(schedule_iterator) = lru_cache.get(&indiscriminate_expression) {
                return schedule_iterator.clone();
            }
            let taskschedule = Schedule::from_str(&indiscriminate_expression)
                .unwrap()
                .upcoming_owned(Utc);
            lru_cache.put(indiscriminate_expression, taskschedule.clone());
            taskschedule
        })
    }
}

impl Task {
    #[inline(always)]
    pub(crate) fn new(
        task_id: u64,
        frequency: FrequencyInner,
        body: SafeBoxFn,
        maximum_running_time: Option<u64>,
        maximun_parallel_runable_num: u64,
    ) -> Task {
        let body = SafeStructBoxedFn(body);
        Task {
            task_id,
            frequency,
            body,
            maximum_running_time,
            cylinder_line: 0,
            valid: true,
            maximun_parallel_runable_num,
        }
    }

    // get SafeBoxFn of Task to call.
    #[inline(always)]
    pub(crate) fn get_body(&self) -> &SafeBoxFn {
        &(self.body).0
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

mod tests {

    #[test]
    fn test_task_valid() {
        use super::{Frequency, Task, TaskBuilder};
        use crate::utils::convenience::functions::create_default_delay_task_handler;
        let mut task_builder = TaskBuilder::default();

        //The third run returns to an invalid state.
        task_builder.set_frequency(Frequency::CountDown(3, "* * * * * * *"));
        let mut task: Task = task_builder
            .spawn(|| create_default_delay_task_handler())
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
            .spawn(|| create_default_delay_task_handler())
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
            .spawn(|| create_default_delay_task_handler())
            .unwrap();

        assert!(task.is_can_running());

        task.set_cylinder_line(1);
        assert!(!task.is_can_running());

        assert!(task.check_arrived());
    }
}
