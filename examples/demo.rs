use delay_timer::delay_timer::DelayTimer;
use delay_timer::timer::task::{frequency, TaskBuilder};
use delay_timer::timer::timer::Timer;
use std::collections::VecDeque;
use std::sync::mpsc::*;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut delay_timer = DelayTimer::new();
    let mut taskBuilder = TaskBuilder::new();
    let body = || println!("task 1 ,1s run");
    taskBuilder.set_frequency(frequency::repeated("* * * * * * *"));
    taskBuilder.set_task_id(1);
    let mut task = taskBuilder.spawn(body);
    delay_timer.add_task(task);

    let mut taskBuilder = TaskBuilder::new();
    let body = || println!("task 2 ,5s run");
    taskBuilder.set_frequency(frequency::repeated("0/5 * * * * * *"));
    taskBuilder.set_task_id(2);
    let mut task = taskBuilder.spawn(body);
    delay_timer.add_task(task);

    let mut taskBuilder = TaskBuilder::new();
    let body = || println!("task 3 ,3s run, altogether 3times");
    taskBuilder.set_frequency(frequency::CountDown(3, "0/3 * * * * * *"));
    taskBuilder.set_task_id(3);
    let mut task = taskBuilder.spawn(body);
    delay_timer.add_task(task);

    loop {
        sleep(Duration::new(1, 0));
    }
}
