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
type TaskReceiver = Receiver<Task>;
type TaskSender = Sender<Task>;
type FnReceiver = Receiver<Box<Fn() + 'static>>;
type FnSender = Sender<Box<Fn() + 'static>>;

//add channel
//explore on time task
// 暴漏一个 接收方，让读取任务
pub struct Timer {
    wheelQueue: Vec<Slot>,
    TaskSender: TaskSender,
    secondHand: usize,
}

//不在调度者里面执行任务，不然时间会不准
//just provice api and struct ,less is more.
impl Timer {
    pub fn new(TaskSender: TaskSender) -> Self {
        Timer {
            wheelQueue: Vec::with_capacity(DEFAULT_TIMER_SLOT_COUNT as usize),
            TaskSender,
            secondHand: 0,
        }
    }

    pub fn init(&mut self) {
        for _ in 0..DEFAULT_TIMER_SLOT_COUNT {
            self.wheelQueue.push(Slot::new());
        }
    }

    //add task to wheelQueue  slot
    pub fn add_task(&mut self, mut task: Task) {
        let exec_time = task.get_next_exec_timestamp() as u64;
        let time_seed: usize = (exec_time - get_timestamp()) as usize;
        let slot_seed: usize = (time_seed as usize) % DEFAULT_TIMER_SLOT_COUNT;

        task.set_cylinder_line((time_seed / DEFAULT_TIMER_SLOT_COUNT) as u32);

        println!(
            "task_id:{}, next_time:{}, slot_seed:{}",
            task.task_id, exec_time, slot_seed
        );
        self.wheelQueue[slot_seed].add_task(task);
    }

    pub fn next_position(&mut self) {
        self.secondHand = self.secondHand + 1 % DEFAULT_TIMER_SLOT_COUNT;
    }

    ///here,I wrote so poorly, greet you give directions.
    pub fn elapse(&mut self) {
        //not runing 1s ,Duration - runing time
        //sleep  ,then loop
        //if that overtime , i run it not block
        let one_second = Duration::new(1, 0);

        loop {
            let instant = Instant::now();
            let task_ids = self.wheelQueue[self.secondHand].arrival_time_tasks();
            println!("run : go : go: {}", self.secondHand);
            for task_id in task_ids {
                let mut task = self.wheelQueue[self.secondHand]
                    .remove_task(task_id)
                    .unwrap();
                (task.body)();

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

    fn schedule(&mut self) {}

    pub fn start(&mut self) {}

    //发送给Timer 让他去决定插到哪里
    //返回一组 任务ID
    //让调度这去，remove 跟 add
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

// [[task1,quan=>1,emun=>repeat,quality=>50s,]]
