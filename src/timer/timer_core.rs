// use super::event_handle;
use super::event_handle::{SharedTaskFlagMap, SharedTaskWheel};
pub(crate) use super::runtime_trace::task_handle::DelayTaskHandlerBox;
use super::runtime_trace::task_handle::DelayTaskHandlerBoxBuilder;
pub(crate) use super::slot::Slot;
pub(crate) use super::task::Task;
pub(crate) use smol::channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
///timer,时间轮
/// 未来我会将这个库，实现用于rust-cron
/// someting i will wrote chinese ,someting i will wrote english
/// I wanna wrote bilingual language show doc...
/// there ara same content.
use snowflake::SnowflakeIdBucket;

pub(crate) use super::task::TaskMark;
use smol::Timer as SmolTimer;
use std::sync::{
    atomic::{
        AtomicU64,
        Ordering::{Relaxed, Release},
    },
    Arc,
};
use std::time::{Duration, Instant, SystemTime};

pub(crate) const DEFAULT_TIMER_SLOT_COUNT: u64 = 3600;

pub(crate) type TimerEventSender = AsyncSender<TimerEvent>;
pub(crate) type TimerEventReceiver = AsyncReceiver<TimerEvent>;
pub(crate) type SencondHand = Arc<AtomicU64>;

//我需要将 使用task_id关联任务，放到一个全局的hash表
//两个作用，task_id 跟 Task 一一对应
//在hash表上会存着，Task当前处在的Slot
//TODO: Maybe that's can optimize.(We can add/del/set TaskMark in Timer.async_schedule)
//TASKMAP is use to storage all TaskMark for check.

//timer-async_schedule 负责往里面写/改数据， handel-event 负责删除

//TODO:warning: large size difference between variants
pub(crate) enum TimerEvent {
    StopTimer,
    AddTask(Box<Task>),
    RemoveTask(u64),
    CancelTask(u64, i64),
    StopTask(u64),
    AppendTaskHandle(u64, DelayTaskHandlerBox),
}

//add channel
//explore on time task
// 暴漏一个 接收方，让读取任务
pub struct Timer {
    wheel_queue: SharedTaskWheel,
    task_flag_map: SharedTaskFlagMap,
    timer_event_sender: TimerEventSender,
    second_hand: SencondHand,
    status_report_sender: Option<AsyncSender<i32>>,
}

//不在调度者里面执行任务，不然时间会不准
//just provice api and struct ,less is more.
impl Timer {
    pub(crate) fn new(
        wheel_queue: SharedTaskWheel,
        task_flag_map: SharedTaskFlagMap,
        timer_event_sender: TimerEventSender,
    ) -> Self {
        Timer {
            wheel_queue,
            task_flag_map,
            timer_event_sender,
            second_hand: Arc::new(AtomicU64::new(0)),
            status_report_sender: None,
        }
    }

    pub(crate) fn set_status_report_sender(&mut self, sender: AsyncSender<i32>) {
        self.status_report_sender = Some(sender);
    }

    //TODO:features append fn put there.
    pub(crate) fn features_append_fn(&mut self, sender: AsyncSender<i32>) {
        #[cfg(feature = "status-report")]
        fn report(&mut self, record: i32) {
            // async.sender.send(record);
        }

        #[cfg(feature = "status-report")]
        self.report(1);
    }

    pub fn next_position(&mut self) -> u64 {
        self.second_hand
            .fetch_update(Release, Relaxed, |x| {
                Some((x + 1) % DEFAULT_TIMER_SLOT_COUNT)
            })
            .unwrap_or_else(|e| e)
    }

    ///here,I wrote so poorly, greet you give directions.
    /// schedule：
    ///    first. add ||　remove task 。
    ///    second.spawn task .
    ///    thirid sleed 1- (run duration).

    pub async fn async_schedule(&mut self) {
        //not runing 1s ,Duration - runing time
        //sleep  ,then loop
        //if that overtime , i run it not block

        let mut now;
        let mut when;
        let mut second_hand;
        let mut timestamp;

        //TODO:auto-get nodeid and machineid.
        let mut snowflakeid_bucket = SnowflakeIdBucket::new(1, 1);
        loop {
            second_hand = self.next_position();
            now = Instant::now();
            when = now + Duration::from_secs(1);
            timestamp = get_timestamp();

            let task_ids;

            {
                task_ids = self
                    .wheel_queue
                    .get_mut(&second_hand)
                    .unwrap()
                    .value_mut()
                    .arrival_time_tasks();
            }

            println!("timer-core:Timer-second_hand: {}", second_hand);
            for task_id in task_ids {
                let task_option: Option<Task>;

                {
                    task_option = self
                        .wheel_queue
                        .get_mut(&second_hand)
                        .unwrap()
                        .value_mut()
                        .remove_task(task_id);
                }

                if let Some(mut task) = task_option {
                    let task_handler_box = (task.body)();

                    let delay_task_handler_box_builder = DelayTaskHandlerBoxBuilder::default();
                    let tmp_task_handler_box = delay_task_handler_box_builder
                        .set_task_id(dbg!(task_id))
                        .set_record_id(dbg!(snowflakeid_bucket.get_id()))
                        .set_start_time(dbg!(timestamp))
                        .set_end_time(dbg!(task.get_maximum_running_time(timestamp)))
                        .spawn(task_handler_box);

                    let task_valid = task.down_count_and_set_vaild();
                    if !dbg!(task_valid) {
                        drop(task);
                        continue;
                    }

                    //下一次执行时间
                    let task_excute_timestamp = task.get_next_exec_timestamp();

                    //时间差+当前的分针
                    //比如 时间差是 7260，目前分针再 3599，7260+3599 = 10859
                    //， 从 当前 3599 走碰见三次，再第59个格子
                    let step = task_excute_timestamp - timestamp + second_hand;
                    let quan = step / DEFAULT_TIMER_SLOT_COUNT;
                    task.set_cylinder_line(quan);
                    let slot_seed = step % DEFAULT_TIMER_SLOT_COUNT;

                    println!(
                        "timer-core:task_id:{}, next_time:{}, slot_seed:{}, quan:{}",
                        task.task_id, step, slot_seed, quan
                    );

                    self.timer_event_sender
                        .send(TimerEvent::AppendTaskHandle(task_id, tmp_task_handler_box))
                        .await
                        .unwrap_or_else(|e| println!("{}", e));

                    {
                        self.task_flag_map
                            .get_mut(&task_id)
                            .unwrap()
                            .value_mut()
                            .set_slot_mark(slot_seed);
                    }

                    {
                        self.wheel_queue
                            .get_mut(&slot_seed)
                            .unwrap()
                            .value_mut()
                            .add_task(task);
                    }
                }
            }

            SmolTimer::at(when).await;
        }
    }
}

pub fn get_timestamp() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
