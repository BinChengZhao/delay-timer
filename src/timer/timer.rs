use super::slot::Slot;
///timer,时间轮
/// 未来我会将这个库，实现用于rust-cron
/// someting i will wrote chinese ,someting i will wrote english
/// I wanna wrote bilingual language show doc...
/// there ara same content.
/// FIXME: try it.
///
use super::task::Task;
use cron_clock::schedule::Schedule;

use std::collections::{LinkedList, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::SystemTime;

const DEFAULT_TIMER_SLOT_COUNT: usize = 3600;
const DEFAULT_SLOT_TASK_COUNT: usize = 10;

type TimeExpression = Schedule;
type TaskReceiver = Receiver<Task>;
type TaskSender = Sender<Task>;
type FnReceiver = Receiver<Box<Fn() + 'static>>;
type FnSender = Sender<Box<Fn() + 'static>>;

//add channel
//explore on time task
// 暴漏一个 接收方，让读取任务
pub struct Timer {
    wheelQueue: VecDeque<Slot>,
    TaskSender: TaskSender,
}

//just provice api and struct ,less is more.
impl Timer {
    pub fn new(TaskSender: TaskSender) -> Self {
        Timer {
            wheelQueue: VecDeque::with_capacity(3600),
            TaskSender,
        }
    }

    pub fn init(&mut self) {
        // for _ in 0..DEFAULT_TIMER_SLOT_COUNT {
        //     self.wheelQueue.push_back(Slot::new());
        // }
    }

    pub fn add_task(&mut self, task: Task) {
        //task.timestamp()
        // task.frequency;
        // self.wheelQueue.push_back(task);
    }

    ///here,I wrote so poorly, greet you give directions.
    fn elapse(&mut self) {
        // instant  1s
        //runing
        //取出任务，丢到管道
        //重新整梨及时，task 找对用的slot
        //not runing 1s ,Duration - runing time
        //sleep  ,then loop
        //if that overtime , i run it not block
    }

    fn schedule(&mut self) {}

    pub fn start(&self) {}

    pub fn get_timestamp() -> u64 {
        let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };

        timestamp
    }
}

//wheelQueue  里面的结构，维护一个数组， 第一个是最快要执行的
//每个轮子上，标记还有几圈执行，跟它的任务  ，与它的执行频次
//调度完之后插到新的 slot 中，并标记好圈数

// [[task1,quan=>1,emun=>repeat,quality=>50s,]]

trait TimerTrait {
    fn add_task(&mut self, task: i32) {
        //   todo!();
    }
}
