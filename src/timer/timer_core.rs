use super::runtime_trace::task_handle::{DelayTaskHandlerBoxBuilder, TaskTrace};
use super::slot::Slot;
use super::task::Task;
use async_channel::{Receiver as AsyncReceiver, RecvError, Sender as AsyncSender};
///timer,时间轮
/// 未来我会将这个库，实现用于rust-cron
/// someting i will wrote chinese ,someting i will wrote english
/// I wanna wrote bilingual language show doc...
/// there ara same content.
use snowflake::SnowflakeIdBucket;

use super::task::TaskMark;
use anyhow::Result;
use async_mutex::Mutex as AsyncMutex;
use smol::Timer as SmolTimer;
use std::collections::HashMap;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant, SystemTime};

const DEFAULT_TIMER_SLOT_COUNT: usize = 3600;

pub type TimerEventSender = AsyncSender<TimerEvent>;
pub type TimerEventReceiver = AsyncReceiver<TimerEvent>;

//我需要将 使用task_id关联任务，放到一个全局的hash表
//两个作用，task_id 跟 Task 一一对应
//在hash表上会存着，Task当前处在的Slot
//TODO: Maybe that's can optimize.(We can add/del/set TaskMark in Timer.async_schedule)
//TASKMAP is use to storage all TaskMark for check.

//timer-async_schedule 负责往里面写/改数据， handel-event 负责删除
lazy_static! {
    pub static ref TASKMAP: AsyncMutex<HashMap<u32, TaskMark>> = {
        let m = HashMap::new();
        AsyncMutex::new(m)
    };
}

//TODO:warning: large size difference between variants
pub enum TimerEvent {
    StopTimer,
    AddTask(Box<Task>),
    // Wating handel evet separate from Timer, add it.... because Timer haven't Sender...
    // UpdateTaskSlotMark(u32, usize),
    RemoveTask(u32),
    CancelTask(u32, i64),
    StopTask(u32),
}

//add channel
//explore on time task
// 暴漏一个 接收方，让读取任务
//TODO: Maybe AsyncMutex<Timer.wheel_queue> for handle_event.
pub struct Timer {
    wheel_queue: Vec<Slot>,
    timer_event_receiver: TimerEventReceiver,
    second_hand: usize,
    task_trace: TaskTrace,
    status_report_sender: Option<AsyncSender<i32>>,
}

//不在调度者里面执行任务，不然时间会不准
//just provice api and struct ,less is more.
impl Timer {
    pub fn new(timer_event_receiver: TimerEventReceiver) -> Self {
        let mut timer = Timer {
            wheel_queue: Vec::with_capacity(DEFAULT_TIMER_SLOT_COUNT as usize),
            timer_event_receiver,
            second_hand: 0,
            task_trace: TaskTrace::default(),
            status_report_sender: None,
        };

        timer.init();
        timer
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
    fn init(&mut self) {
        for _ in 0..DEFAULT_TIMER_SLOT_COUNT {
            self.wheel_queue.push(Slot::new());
        }
    }

    //_handle_event
    //TODO: Maybe can package that in a delayTimeTask or smolTask... Tinking....
    //if package smolTask for it, impl Futuren for this can auto-work.
    async fn handle_event(&mut self) {
        let mut event_result;
        let mut events: Vec<TimerEvent> = Vec::with_capacity(256);

        loop {
            // TODO: recv can happen error.
            // waiting separate handle_event using recv().await....
            // Note－recv :　If the channel is empty, this method waits until there is a message.
            event_result = self.timer_event_receiver.try_recv();
            match event_result {
                Ok(event_value) => {
                    events.push(event_value);
                }

                // should report that error.
                Err(_) => break,
            }
        }

        //TODO:CancelTask, Is cancel once when task is running;
        //I should storage processChild in somewhere, When cancel event hanple i will kill child.

        //TODO:
        //cancel is exit running task.
        //stop is suspension of execution(set vaild).
        //user delete task , node should remove.

        //any `Task`  i can set `valid`  for that stop.

        //if get cancel signal is sync task like 'spwan process' i can kill that, async i can cancel.
        //I think i can save async/sync handel in TaskTrace.
        for event in events {
            match event {
                TimerEvent::StopTimer => {
                    //TODO: DONE all of runing tasks.
                    //clear queue.
                    panic!("i'm stop")
                }
                TimerEvent::AddTask(task) => {
                    if let Some(task_mark) = self.add_task(*task) {
                        Timer::record_task_mark(task_mark).await;
                    }
                }
                TimerEvent::RemoveTask(task_id) => {
                    self.remove_task(task_id).await;
                }
                //TODO: handler error.
                TimerEvent::CancelTask(_task_id, _record_id) => {
                    self.cancel_task(_task_id, _record_id);
                }
                TimerEvent::StopTask(_task_id) => todo!(),
            };
        }
    }

    //add task to wheel_queue  slot
    fn add_task(&mut self, mut task: Task) -> Option<TaskMark> {
        let exec_time = task.get_next_exec_timestamp() as u64;
        println!(
            "task_id:{}, next_time:{}, get_timestamp:{}",
            task.task_id,
            exec_time,
            get_timestamp()
        );
        //TODO:exec_time IS LESS THAN TIMESTAMP.
        let time_seed: usize = (exec_time - get_timestamp()) as usize;
        let slot_seed: usize = (time_seed as usize) % DEFAULT_TIMER_SLOT_COUNT;

        task.set_cylinder_line((time_seed / DEFAULT_TIMER_SLOT_COUNT) as u32);

        println!(
            "task_id:{}, next_time:{}, slot_seed:{}",
            task.task_id, exec_time, slot_seed
        );
        //copy task_id..
        let task_id = task.task_id;

        self.wheel_queue[slot_seed].add_task(task);

        Some(TaskMark::new(task_id, slot_seed))
    }

    pub fn cancel_task(&mut self, task_id: u32, record_id: i64) -> Option<Result<()>> {
        self.task_trace.quit_one_task_handler(task_id, record_id)
    }

    pub fn next_position(&mut self) {
        self.second_hand += 1 % DEFAULT_TIMER_SLOT_COUNT;
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

        //TODO:auto-get nodeid and machineid.
        let mut snowflakeid_bucket = SnowflakeIdBucket::new(1, 1);
        loop {
            now = Instant::now();
            when = now + Duration::from_secs(1);
            self.handle_event().await;
            let task_ids = self.wheel_queue[self.second_hand].arrival_time_tasks();
            println!("Timer-second_hand: {}", self.second_hand);
            for task_id in task_ids {
                let mut task = self.wheel_queue[self.second_hand]
                    .remove_task(task_id)
                    .unwrap();

                //TODO:Task should run in another where.
                //因为有可能是，计算量大的闭包。。
                //TODO:_tmp build handler box
                let mut delay_task_handler_box_builder = DelayTaskHandlerBoxBuilder::default();
                delay_task_handler_box_builder.set_task_id(task_id);

                delay_task_handler_box_builder.set_record_id(snowflakeid_bucket.get_id());

                let task_handler_box = (task.body)();
                let _tmp_task_handler_box = delay_task_handler_box_builder.spawn(task_handler_box);
                self.task_trace.insert(task_id, _tmp_task_handler_box);

                let task_valid = task.down_count_and_set_vaild();
                println!("task_id:{}, valid:{}", task.task_id, task_valid);
                if !task_valid {
                    drop(task);
                    continue;
                }

                //下一次执行时间
                let timestamp = task.get_next_exec_timestamp() as usize;

                //时间差+当前的分针
                //比如 时间差是 7260，目前分针再 3599，7260+3599 = 10859
                //， 从 当前 3599 走碰见三次，再第59个格子
                let step = timestamp - (get_timestamp() as usize) + self.second_hand;
                let quan = step / DEFAULT_TIMER_SLOT_COUNT;
                task.set_cylinder_line(quan as u32);
                let slot_seed = step % DEFAULT_TIMER_SLOT_COUNT;
                println!(
                    "task_id:{}, next_time:{}, slot_seed:{}, quan:{}",
                    task.task_id, step, slot_seed, quan
                );
                //TODO: waiting for separate....
                Timer::update_task_mark(task.task_id, slot_seed);

                self.wheel_queue[slot_seed].add_task(task);
            }

            SmolTimer::at(when).await;
            self.next_position();
        }
    }

    //TODO:Don't forget that separate-fn-handle_event-in-timer when you have time.
    async fn record_task_mark(task_mark: TaskMark) {
        let mut m = TASKMAP.lock().await;
        m.insert(task_mark.task_id, task_mark);
    }

    async fn update_task_mark(task_id: u32, slot_mark: usize) {
        let mut m = TASKMAP.lock().await;
        m.entry(task_id).and_modify(|e| e.set_slot_mark(slot_mark));
    }

    async fn remove_task(&mut self, task_id: u32) -> Option<Task> {
        let mut m = TASKMAP.lock().await;

        let task_mark: Option<&TaskMark> = m.get(&task_id);

        if let Some(t) = task_mark {
            let slot_mark = t.get_slot_mark();
            return self.wheel_queue[slot_mark as usize].remove_task(task_id);
        }
        None
    }
}

pub fn get_timestamp() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
