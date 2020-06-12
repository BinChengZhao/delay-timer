#[allow(dead_code)]
pub mod timer;

#[macro_use]
extern crate lazy_static;

//先写出来库，未来在cron中 基于async-std,把 timer的调度给包装起来
#[cfg(test)]
mod tests {
    use crate::timer;
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
        let (send, recv) = channel();
        let mut timer = Timer::new(send);

        // timer.add_task(1);
        // timer.add_task(2);
        // timer.add_task(2);

        //like  timer.schedule(sender);
        //let   new::excutor(reciver);
        //excutor can many

        //timer要锁住，支持多个线程使用
        //一个线程使用timer不断吐给管道任务
        //一个线程，已事件驱动的方式运行，当有需要时，给timer增加任务
        //后续支持取消任务
        //每个任务有一个ID，支持通过ID取消
        timer.schedule();
        //after schedule, also can add_task
        //so new timer should give a sender
        //in schedule, it will steady supply task to channel
        //have one excutor run  task

        // timer.add_task(1);
        // timer.add_task(2);
        // timer.add_task(2);

        //

        assert_eq!("task1 resut", "expect task1 resut");
        assert_eq!("task2 resut", "expect task2 resut");
        assert_eq!("task3 resut", "expect task3 resut");
    }

    #[test]
    fn add_task() {
        let (send, recv) = channel();
        let mut timer = Timer::new(send);
        // timer.add_task(1);
        // timer.add_task(2);
        // timer.add_task(3);

        timer.schedule();

        assert_eq!("task1 resut", "expect task1 resut");
        assert_eq!("task2 resut", "expect task2 resut");
        assert_eq!("task3 resut", "expect task3 resut");
    }
}
