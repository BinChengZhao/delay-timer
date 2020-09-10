// use super::event_handle;
use super::event_handle::{SharedTaskFlagMap, SharedTaskWheel};
pub(crate) use super::runtime_trace::task_handle::DelayTaskHandlerBox;
use super::runtime_trace::task_handle::{DelayTaskHandlerBoxBuilder, TaskTrace};
pub(crate) use super::slot::Slot;
pub(crate) use super::task::Task;
pub(crate) use smol::channel::{Receiver as AsyncReceiver, RecvError, Sender as AsyncSender};
///timer,时间轮
/// 未来我会将这个库，实现用于rust-cron
/// someting i will wrote chinese ,someting i will wrote english
/// I wanna wrote bilingual language show doc...
/// there ara same content.
use snowflake::SnowflakeIdBucket;

pub(crate) use super::task::TaskMark;
use anyhow::Result;
use smol::Timer as SmolTimer;
use std::collections::HashMap;
use std::sync::{
    atomic::{
        AtomicUsize,
        Ordering::{Acquire, Relaxed, Release},
    },
    Arc,
};
use std::time::{Duration, Instant, SystemTime};

pub(crate) const DEFAULT_TIMER_SLOT_COUNT: usize = 3600;

pub(crate) type TimerEventSender = AsyncSender<TimerEvent>;
pub(crate) type TimerEventReceiver = AsyncReceiver<TimerEvent>;
pub(crate) type SencondHand = Arc<AtomicUsize>;

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
    // Wating handel evet separate from Timer, add it.... because Timer haven't Sender...
    // UpdateTaskSlotMark(u32, usize),
    RemoveTask(usize),
    CancelTask(usize, i64),
    StopTask(usize),
    AppendTaskHandle(usize, DelayTaskHandlerBox),
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
            second_hand: Arc::new(AtomicUsize::new(0)),
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

    pub fn next_position(&mut self) -> usize {
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

        //TODO:auto-get nodeid and machineid.
        let mut snowflakeid_bucket = SnowflakeIdBucket::new(1, 1);
        loop {
            second_hand = self.next_position();
            now = Instant::now();
            when = now + Duration::from_secs(1);

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
                    let mut delay_task_handler_box_builder = DelayTaskHandlerBoxBuilder::default();
                    delay_task_handler_box_builder.set_task_id(task_id);

                    delay_task_handler_box_builder.set_record_id(snowflakeid_bucket.get_id());

                    let task_handler_box = (task.body)();
                    let _tmp_task_handler_box =
                        delay_task_handler_box_builder.spawn(task_handler_box);

                    let task_valid = task.down_count_and_set_vaild();
                    println!("timer-core:task_id:{}, valid:{}", task.task_id, task_valid);
                    if !task_valid {
                        drop(task);
                        continue;
                    }

                    //下一次执行时间
                    let timestamp = task.get_next_exec_timestamp();

                    //时间差+当前的分针
                    //比如 时间差是 7260，目前分针再 3599，7260+3599 = 10859
                    //， 从 当前 3599 走碰见三次，再第59个格子
                    let step = timestamp - get_timestamp() + second_hand;
                    let quan = step / DEFAULT_TIMER_SLOT_COUNT;
                    task.set_cylinder_line(quan);
                    let slot_seed = step % DEFAULT_TIMER_SLOT_COUNT;

                    println!(
                        "timer-core:task_id:{}, next_time:{}, slot_seed:{}, quan:{}",
                        task.task_id, step, slot_seed, quan
                    );

                    self.timer_event_sender
                        .send(TimerEvent::AppendTaskHandle(task_id, _tmp_task_handler_box))
                        .await;

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

pub fn get_timestamp() -> usize {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs() as usize,
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
