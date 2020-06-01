///timer,时间轮
/// 未来我会将这个库，实现用于rust-cron
/// someting i will wrote chinese ,someting i will wrote english
/// I wanna wrote bilingual language show doc...
/// there ara same content.
/// FIXME: try it.
/// 
/// 
use std::collections::VecDeque;
pub struct Timer {
    wheelQueue: VecDeque<i32>,
}

struct Slot {
    tasks: Vec<task>,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            wheelQueue: VecDeque::with_capacity(3600),
        }
    }

    pub fn add_task(&mut self, task: task) {
        // task.frequency;
        // self.wheelQueue.push_back(task);
    }

    ///here,I wrote so poorly, greet you give directions.
    fn elapse(&mut self) {
        // instant  1s
        //runing
        //not runing 1s ,Duration - runing time
        //sleep  ,then loop
        //if that overtime , i run it not block
    }

    fn schedule(&self) {}

    pub fn start(&self) {}
}

//TODO: 这里得改
//FIXME:
pub struct repeated_period(u32, u32, u32, u32, u32);
pub enum Frequency {
    Once,
    repeated(repeated_period),
    CountDown(u32, repeated_period),
}

pub struct task {
    frequency: Frequency,
    body: Box<Fn() + 'static>,
    cylinder_line: u32,
}

impl task {
   pub fn new(frequency: Frequency, body: Box<Fn() + 'static>, cylinder_line: u32) -> Self {
        task {
            frequency,
            body,
            cylinder_line,
        }
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

#[cfg(test)]
mod tests {
    use crate::Timer;
    use std::collections::VecDeque;

    #[test]
    fn check() {
        let mut v = VecDeque::with_capacity(3600);
        v.push_back(1);
        v.push_back(2);
        v.push_back(3);

        let mut j = 0;
        loop {
            if j == 5 {
                break;
            }

            for i in v.iter() {
                println!("{}", i);
            }

            j += 1;
        }
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn run() {
        let mut timer = Timer::new();
        timer.add_task(1);
        timer.add_task(2);
        timer.add_task(2);

        timer.schedule();

        assert_eq!("task1 resut", "expect task1 resut");
        assert_eq!("task2 resut", "expect task2 resut");
        assert_eq!("task3 resut", "expect task3 resut");
    }

    #[test]
    fn add_task() {
        let mut timer = Timer::new();
        timer.add_task(1);
        timer.add_task(2);
        timer.add_task(3);

        timer.schedule();

        assert_eq!("task1 resut", "expect task1 resut");
        assert_eq!("task2 resut", "expect task2 resut");
        assert_eq!("task3 resut", "expect task3 resut");
    }
}
