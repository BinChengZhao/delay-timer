pub mod delay_timer;
#[allow(dead_code)]
pub mod timer;

#[macro_use]
extern crate lazy_static;

mod example {

    //TODO: TDD.
    fn demo() {

        //DelayTimer::new() -> Timer;
        // fn get_delay_timer() -> Timer {
        //     let (r, t) = channel();

        //     let schedule = {r};

        // thread::spawn(move smol::run(async{
        //     schedule.schedule();
        // })
        // );

        // let reactor = {s};

        // return reactor;
        // }

        //delay_timer is autostart.
        // let delay_timer = get_delay_timer();
        //delay_timer.add_task(task);
        //delay_timer.set_task(task); //修改任务--可以delay做
        //delay_timer.remove_task(task);
        //delay_timer.stop();
    }
}

//先写出来库，未来在cron中 基于smol,把 timer的调度给包装起来
// #[cfg(test)]
// mod tests {
//     use crate::timer::slot::Slot;
//     use crate::timer::timer::Timer;
//     use std::collections::VecDeque;
//     use std::sync::mpsc::*;

//     use crate::timer::task::*;
//     #[test]
//     fn test_build_task() {
//         let mut taskBuilder = TaskBuilder::new();
//         let body = || println!("{}", 1 + 1);
//         taskBuilder.set_frequency(frequency::repeated("* * * * * * *"));
//         taskBuilder.set_task_id(123);
//         let mut task = taskBuilder.spawn(async { body });

//         println!("{}", task.is_valid());
//         println!("{}", task.get_next_exec_timestamp());
//         println!("{}", task.get_next_exec_timestamp());
//     }

//     #[test]
//     fn test_timer() {
//         let (s, r) = channel();
//         let mut timer = Timer::new(s);
//         timer.init();
//         let mut taskBuilder = TaskBuilder::new();
//         let body = || println!("task 1 ,1s run");
//         taskBuilder.set_frequency(frequency::repeated("* * * * * * *"));
//         taskBuilder.set_task_id(1);
//         let mut task = taskBuilder.spawn(async { body });
//         timer.add_task(task);

//         // let mut taskBuilder = TaskBuilder::new();
//         // let body = || println!("task 2 ,5s run");
//         // taskBuilder.set_frequency(frequency::repeated("0/5 * * * * * *"));
//         // taskBuilder.set_task_id(2);
//         // let mut task = taskBuilder.spawn(async { body });
//         // timer.add_task(task);

//         // let mut taskBuilder = TaskBuilder::new();
//         // let body = || println!("task 3 ,3s run, altogether 3times");
//         // taskBuilder.set_frequency(frequency::CountDown(3, "0/3 * * * * * *"));
//         // taskBuilder.set_task_id(3);
//         // let mut task = taskBuilder.spawn(async { body });
//         // timer.add_task(task);

//         timer.schedule();
//     }

//     #[test]
//     fn test_slot() {
//         let mut slot = Slot::new();
//         let mut taskBuilder = TaskBuilder::new();
//         let body = || println!("task 1 ,1s run");
//         taskBuilder.set_frequency(frequency::repeated("* * * * * * *"));
//         taskBuilder.set_task_id(1);
//         let mut task = taskBuilder.spawn(async { body });
//         task.set_cylinder_line(2);

//         // slot.add_task(task);
//         // let mut taskBuilder = TaskBuilder::new();
//         // let body = || println!("task 2 ,5s run");
//         // taskBuilder.set_frequency(frequency::repeated("0/5 * * * * * *"));
//         // taskBuilder.set_task_id(2);
//         // let mut task = taskBuilder.spawn(async { body });
//         // task.set_cylinder_line(2);
//         // slot.add_task(task);

//         // println!("kong:{:?}", slot.arrival_time_tasks());

//         // let mut taskBuilder = TaskBuilder::new();
//         // let body = || println!("task 3 ,5s run");
//         // taskBuilder.set_frequency(frequency::repeated("0/5 * * * * * *"));
//         // taskBuilder.set_task_id(3);
//         // let mut task = taskBuilder.spawn(async { body });
//         // task.set_cylinder_line(2);
//         // slot.add_task(task);

//         println!("U3：{:?}", slot.arrival_time_tasks());
//         println!("U3+：{:?}", slot.arrival_time_tasks());
//     }

//     #[test]
//     fn it_works() {
//         assert_eq!(2 + 2, 4);
//     }
// }
