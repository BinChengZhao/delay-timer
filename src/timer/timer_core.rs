// use super::event_handle;
use super::event_handle::{SharedTaskFlagMap, SharedTaskWheel};
pub(crate) use super::runtime_trace::task_handle::DelayTaskHandlerBox;
use super::runtime_trace::task_handle::DelayTaskHandlerBoxBuilder;
pub(crate) use super::slot::Slot;
pub(crate) use super::task::Task;
pub(crate) use smol::channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
///timer,时间轮
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

//warning: large size difference between variants
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
        second_hand: SencondHand,
    ) -> Self {
        Timer {
            wheel_queue,
            task_flag_map,
            timer_event_sender,
            second_hand,
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

    //TODO:读取当前slot时，提前+1 ，让event_handle 怎样都可以插入到之后slot
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
                let mut slot_mut = self.wheel_queue.get_mut(&second_hand).unwrap();

                task_ids = slot_mut.value_mut().arrival_time_tasks();
            }

            println!("timer-core:Timer-second_hand: {}", second_hand);
            for task_id in task_ids {
                let task_option: Option<Task>;

                {
                    let mut slot_mut = self.wheel_queue.get_mut(&second_hand).unwrap();

                    task_option = slot_mut.value_mut().remove_task(task_id);
                }

                if let Some(mut task) = task_option {
                    let task_handler_box = (task.body)();

                    let delay_task_handler_box_builder = DelayTaskHandlerBoxBuilder::default();
                    let tmp_task_handler_box = delay_task_handler_box_builder
                        .set_task_id(task_id)
                        .set_record_id(snowflakeid_bucket.get_id())
                        .set_start_time(timestamp)
                        .set_end_time(task.get_maximum_running_time(timestamp))
                        .spawn(task_handler_box);

                    let task_valid = task.down_count_and_set_vaild();
                    if !task_valid {
                        drop(task);
                        continue;
                    }

                    //下一次执行时间
                    let task_excute_timestamp = task.get_next_exec_timestamp();

                    //时间差+当前的分针
                    //比如 时间差是 7260，目前分针再 3599，7260+3599 = 10859
                    //， 从 当前 3599 走碰见三次，再第59个格子
                    let step = task_excute_timestamp
                        .checked_sub(timestamp)
                        .unwrap_or_else(|| task.task_id % DEFAULT_TIMER_SLOT_COUNT)
                        + second_hand;
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
                        let mut slot_mut = self.wheel_queue.get_mut(&slot_seed).unwrap();

                        slot_mut.value_mut().add_task(task);
                    }

                    {
                        let mut task_flag_map = self.task_flag_map.get_mut(&task_id).unwrap();

                        task_flag_map.value_mut().set_slot_mark(slot_seed);
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
