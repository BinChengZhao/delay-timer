#[allow(dead_code)]
pub mod timer;

#[macro_use]
extern crate lazy_static;

//先写出来库，未来在cron中 基于async-std,把 timer的调度给包装起来
#[cfg(test)]
mod tests {
    use crate::timer::timer::Timer;
    use crate::timer::slot::Slot;
    use std::collections::VecDeque;
    use std::sync::mpsc::*;

    use crate::timer::task::*;
    #[test]
    fn test_build_task() {
        let mut taskBuilder = TaskBuilder::new();
        let body = || println!("{}", 1 + 1);
        taskBuilder.set_frequency(frequency::repeated("* * * * * * *"));
        taskBuilder.set_task_id(123);
        let mut task = taskBuilder.spawn(body);

        println!("{}", task.is_valid());
        println!("{}", task.get_next_exec_timestamp());
        println!("{}", task.get_next_exec_timestamp());
    }

    #[test]
    fn test_timer() {
        let (s, r) = channel();
        let mut timer = Timer::new(s);
        timer.init();
        let mut taskBuilder = TaskBuilder::new();
        let body = || println!("task 1 ,1s run");
        taskBuilder.set_frequency(frequency::repeated("* * * * * * *"));
        taskBuilder.set_task_id(1);
        let mut task = taskBuilder.spawn(body);
        timer.add_task(task);
        let mut taskBuilder = TaskBuilder::new();
        let body = || println!("task 2 ,5s run");
        taskBuilder.set_frequency(frequency::repeated("0/5 * * * * * *"));
        taskBuilder.set_task_id(2);
        let mut task = taskBuilder.spawn(body);
        timer.add_task(task);
        timer.elapse();
    }



    #[test]
    fn test_slot() {
        let mut slot = Slot::new();
        let mut taskBuilder = TaskBuilder::new();
        let body = || println!("task 1 ,1s run");
        taskBuilder.set_frequency(frequency::repeated("* * * * * * *"));
        taskBuilder.set_task_id(1);
        let mut task = taskBuilder.spawn(body);
        task.set_cylinder_line(2);

        slot.add_task(task);
        let mut taskBuilder = TaskBuilder::new();
        let body = || println!("task 2 ,5s run");
        taskBuilder.set_frequency(frequency::repeated("0/5 * * * * * *"));
        taskBuilder.set_task_id(2);
        let mut task = taskBuilder.spawn(body);
        task.set_cylinder_line(2);
        slot.add_task(task);

        println!("kong:{:?}", slot.arrival_time_tasks());


        let mut taskBuilder = TaskBuilder::new();
        let body = || println!("task 3 ,5s run");
        taskBuilder.set_frequency(frequency::repeated("0/5 * * * * * *"));
        taskBuilder.set_task_id(3);
        let mut task = taskBuilder.spawn(body);
        task.set_cylinder_line(2);
        slot.add_task(task);

        println!("U3：{:?}", slot.arrival_time_tasks());
        println!("U3+：{:?}", slot.arrival_time_tasks());


    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

   
  
}
