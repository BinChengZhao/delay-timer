use delay_timer::delay_timer::DelayTimer;
use delay_timer::timer::task::{frequency, TaskBuilder};
use delay_timer::timer::timer::Timer;
use smol::Timer as SmolTimer;
use std::collections::VecDeque;
use std::sync::mpsc::*;
use std::thread::sleep;
use std::time::Duration;
use surf;

fn main() {
    let mut delay_timer = DelayTimer::new();
    let mut taskBuilder = TaskBuilder::new();
    let body = || println!("task 1 ,1s run");

    use std::fs::OpenOptions;
    use std::process::Command;

    // let file = OpenOptions::new()
    //     .append(true)
    //     .write(true)
    //     .create(true)
    //     .open("./foo.txt");

    taskBuilder.set_frequency(frequency::repeated("* * * * * * *"));
    taskBuilder.set_task_id(1);
    let mut task = taskBuilder.spawn(body);
    delay_timer.add_task(task);

    let mut taskBuilder = TaskBuilder::new();
    let body = || println!("task 2 ,5s run");
    let body = || {
        let output = Command::new("php")
            .arg("-v")
            .spawn()
            .expect("Failed to execute command");
    };
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

    let mut taskBuilder = TaskBuilder::new();
    let body = || println!("task 4 ,4s run, altogether 4times");
    taskBuilder.set_frequency(frequency::CountDown(4, "0/4 * * * * * *"));
    taskBuilder.set_task_id(3);
    let mut task = taskBuilder.spawn(body);
    delay_timer.add_task(task);
    loop {
        sleep(Duration::new(1, 0));
    }
}
