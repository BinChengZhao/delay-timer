use super::slot::Slot;
use super::task::Task;
///timer,时间轮
/// 未来我会将这个库，实现用于rust-cron
/// someting i will wrote chinese ,someting i will wrote english
/// I wanna wrote bilingual language show doc...
/// there ara same content.
/// FIXME: try it.
///
use cron_clock::schedule::Schedule;

use std::collections::LinkedList;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};

const DEFAULT_TIMER_SLOT_COUNT: usize = 3600;

type TimeExpression = Schedule;
pub type TimerEventSender = Sender<TimerEvent>;
pub type TimerEventReceiver = Receiver<TimerEvent>;
type TaskReceiver = Receiver<Task>;
type TaskSender = Sender<Task>;
type FnReceiver = Receiver<Box<Fn() + 'static>>;
type FnSender = Sender<Box<Fn() + 'static>>;

pub enum TimerEvent {
    Stop,
    AddTask(Task),
    RemoveTask(u32),
    CancelTask(u32),
}

//add channel
//explore on time task
// 暴漏一个 接收方，让读取任务
pub struct Timer {
    wheelQueue: Vec<Slot>,
    timer_event_receiver: TimerEventReceiver,
    secondHand: usize,
}

//不在调度者里面执行任务，不然时间会不准
//just provice api and struct ,less is more.
impl Timer {
    pub fn new(timer_event_receiver: TimerEventReceiver) -> Self {
        Timer {
            wheelQueue: Vec::with_capacity(DEFAULT_TIMER_SLOT_COUNT as usize),
            timer_event_receiver,
            secondHand: 0,
        }
    }

    pub fn init(&mut self) {
        for _ in 0..DEFAULT_TIMER_SLOT_COUNT {
            self.wheelQueue.push(Slot::new());
        }
    }

    fn _handle_event(&mut self) {
        use std::sync::mpsc::TryRecvError;
        // TODO: recv can happen error.
        let eventResult = self.timer_event_receiver.try_recv();
        let event;
        match eventResult {
            Ok(event_value) => {
                event = event_value;
            }
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => panic!("Disconnected"),
        }

        //TODO:CancelTask, Is cancel once when task is running;
        //I should storage processChild in somewhere, When cancel event hanple i will kill child.
        match event {
            TimerEvent::Stop => panic!("i'm stop"),
            TimerEvent::AddTask(task) => self.add_task(task),
            TimerEvent::RemoveTask(task_id) => self.remove_task(task_id),
            TimerEvent::CancelTask(task_id) => todo!(),
        };
    }

    //add task to wheelQueue  slot
    pub fn add_task(&mut self, mut task: Task) -> Option<Task> {
        let exec_time = task.get_next_exec_timestamp() as u64;
        let time_seed: usize = (exec_time - get_timestamp()) as usize;
        let slot_seed: usize = (time_seed as usize) % DEFAULT_TIMER_SLOT_COUNT;

        task.set_cylinder_line((time_seed / DEFAULT_TIMER_SLOT_COUNT) as u32);

        println!(
            "task_id:{}, next_time:{}, slot_seed:{}",
            task.task_id, exec_time, slot_seed
        );
        self.wheelQueue[slot_seed].add_task(task)
    }

    pub fn next_position(&mut self) {
        self.secondHand = self.secondHand + 1 % DEFAULT_TIMER_SLOT_COUNT;
    }

    ///here,I wrote so poorly, greet you give directions.
    /// schedule：
    ///    first. add ||　remove task 。
    ///    second.spawn task .
    ///    thirid sleed 1- (run duration).
    pub fn schedule(&mut self) {
        //not runing 1s ,Duration - runing time
        //sleep  ,then loop
        //if that overtime , i run it not block
        let one_second = Duration::new(1, 0);

        loop {
            let instant = Instant::now();
            self._handle_event();
            let task_ids = self.wheelQueue[self.secondHand].arrival_time_tasks();
            println!("run : go : go: {}", self.secondHand);
            for task_id in task_ids {
                let mut task = self.wheelQueue[self.secondHand]
                    .remove_task(task_id)
                    .unwrap();

                //TODO:Task should run in another where.
                (task.body)();

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
                let step = timestamp - (get_timestamp() as usize) + self.secondHand;
                let quan = step / DEFAULT_TIMER_SLOT_COUNT;
                task.set_cylinder_line(quan as u32);
                let slot_seed = step % DEFAULT_TIMER_SLOT_COUNT;
                println!(
                    "task_id:{}, next_time:{}, slot_seed:{}, quan:{}",
                    task.task_id, step, slot_seed, quan
                );

                self.wheelQueue[slot_seed].add_task(task);
            }

            sleep(one_second - instant.elapsed());

            self.next_position();
        }
    }

    pub fn remove_task(&mut self, task_id: u32) -> Option<Task> {
        use crate::timer::task::{TaskMark, TASKMAP};

        let mut m = TASKMAP.lock().unwrap();
        let task_mark: Option<&TaskMark> = m.get(&task_id);

        if task_mark.is_some() {
            let t = task_mark.unwrap();
            let slot_mark = t.get_slot_mark();
            return self.wheelQueue[slot_mark as usize].remove_task(task_id);
        }

        None
    }
}

pub fn get_timestamp() -> u64 {
    let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    };

    timestamp
}

//wheelQueue  里面的结构，维护一个数组， 第一个是最快要执行的
//每个轮子上，标记还有几圈执行，跟它的任务  ，与它的执行频次
//调度完之后插到新的 slot 中，并标记好圈数
