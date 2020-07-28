use super::runtime_trace::task_handle::{
    DelayTaskHandlerBox, DelayTaskHandlerBoxBuilder, TaskTrace,
};
use super::slot::Slot;
use super::task::Task;
///timer,时间轮
/// 未来我会将这个库，实现用于rust-cron
/// someting i will wrote chinese ,someting i will wrote english
/// I wanna wrote bilingual language show doc...
/// there ara same content.
use cron_clock::schedule::Schedule;

use smol::Timer as SmolTimer;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};

const DEFAULT_TIMER_SLOT_COUNT: usize = 3600;

type TimeExpression = Schedule;
pub type TimerEventSender = Sender<TimerEvent>;
pub type TimerEventReceiver = Receiver<TimerEvent>;
type TaskReceiver = Receiver<Task>;
type TaskSender = Sender<Task>;
type FnReceiver = Receiver<Box<dyn Fn() + 'static>>;
type FnSender = Sender<Box<dyn Fn() + 'static>>;

//TODO:warning: large size difference between variants
pub enum TimerEvent {
    StopTimer,
    AddTask(Box<Task>),
    RemoveTask(u32),
    CancelTask(u32),
    StopTask(u32),
    //TODO: CAHNGEE.
}

//add channel
//explore on time task
// 暴漏一个 接收方，让读取任务
pub struct Timer {
    wheel_queue: Vec<Slot>,
    timer_event_receiver: TimerEventReceiver,
    second_hand: usize,
}

//不在调度者里面执行任务，不然时间会不准
//just provice api and struct ,less is more.
impl Timer {
    pub fn new(timer_event_receiver: TimerEventReceiver) -> Self {
        let mut timer = Timer {
            wheel_queue: Vec::with_capacity(DEFAULT_TIMER_SLOT_COUNT as usize),
            timer_event_receiver,
            second_hand: 0,
        };

        timer.init();
        timer
    }

    fn init(&mut self) {
        for _ in 0..DEFAULT_TIMER_SLOT_COUNT {
            self.wheel_queue.push(Slot::new());
        }
    }

    //_handle_event
    fn handle_event(&mut self) {
        use std::sync::mpsc::TryRecvError;
        let mut event_result;
        let mut events: Vec<TimerEvent> = Vec::new();

        loop {
            // TODO: recv can happen error.
            event_result = self.timer_event_receiver.try_recv();
            match event_result {
                Ok(event_value) => {
                    events.push(event_value);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("Disconnected"),
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
                TimerEvent::AddTask(task) => self.add_task(*task),
                TimerEvent::RemoveTask(task_id) => self.remove_task(task_id),
                TimerEvent::CancelTask(_task_id) => todo!(),
                TimerEvent::StopTask(_task_id) => todo!(),
            };
        }
    }

    //add task to wheel_queue  slot
    pub fn add_task(&mut self, mut task: Task) -> Option<Task> {
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
        self.wheel_queue[slot_seed].add_task(task)
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

        use smol::Task as SmolTask;
        use std::process::Child;
        //TODO: can't both in there.
        let mut _task_trace = TaskTrace::new();

        let mut now;
        let mut when;
        loop {
            now = Instant::now();
            when = now + Duration::from_secs(1);
            self.handle_event();
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

                let _tmp_task_record_id = task.get_next_exec_timestamp() as u64;
                delay_task_handler_box_builder.set_record_id(_tmp_task_record_id);

                let task_handler_box = (task.body)();
                let _tmp_task_handler_box =delay_task_handler_box_builder.spawn(task_handler_box);
                _task_trace.insert(task_id, _tmp_task_handler_box);


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

                self.wheel_queue[slot_seed].add_task(task);
            }

            SmolTimer::at(when).await;
            self.next_position();
        }
    }

    pub fn remove_task(&mut self, task_id: u32) -> Option<Task> {
        use crate::timer::task::{TaskMark, TASKMAP};

        let m = TASKMAP.lock().unwrap();
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

//wheel_queue  里面的结构，维护一个数组， 第一个是最快要执行的
//每个轮子上，标记还有几圈执行，跟它的任务  ，与它的执行频次
//调度完之后插到新的 slot 中，并标记好圈数
