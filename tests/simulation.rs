#![feature(ptr_internals)]
use delay_timer::{
    cron_clock::{ScheduleIteratorOwned, Utc},
    delay_timer::DelayTimer,
    timer::task::{Frequency, Task, TaskBuilder},
    utils::functions::create_default_delay_task_handler,
};
use std::{
    sync::{
        atomic::{
            AtomicUsize,
            Ordering::{Acquire, Release},
        },
        Arc,
    },
    thread::park_timeout,
    time::Duration,
};

#[test]
fn go_works() {
    let mut delay_timer = DelayTimer::new();
    let share_num = Arc::new(AtomicUsize::new(0));
    let share_num_bunshin = share_num.clone();

    let body = move || {
        share_num_bunshin.fetch_add(1, Release);
        create_default_delay_task_handler()
    };

    let task = TaskBuilder::default()
        .set_frequency(Frequency::CountDown(3, "0/6 * * * * * *"))
        .set_task_id(1)
        .spawn(body);
    delay_timer.add_task(task).unwrap();

    let mut i = 0;

    loop {
        i = i + 1;
        park_timeout(Duration::from_micros(7_100_000));

        //Testing, whether the mission is performing as expected.
        assert_eq!(i, share_num.load(Acquire));

        if i == 3 {
            break;
        }
    }
}

#[test]
fn tests_countdown() {
    let mut delay_timer = DelayTimer::new();
    let share_num = Arc::new(AtomicUsize::new(3));
    let share_num_bunshin = share_num.clone();
    let body = move || {
        share_num_bunshin.fetch_sub(1, Release);
        create_default_delay_task_handler()
    };

    let task = TaskBuilder::default()
        .set_frequency(Frequency::CountDown(3, "0/2 * * * * * *"))
        .set_task_id(1)
        .spawn(body);
    delay_timer.add_task(task).unwrap();

    let mut i = 0;

    loop {
        i = i + 1;
        park_timeout(Duration::from_secs(3));

        if i == 6 {
            //The task runs 3 times, once per second, and after 6 seconds it goes down to 0 at most.
            assert_eq!(0, share_num.load(Acquire));
            break;
        }
    }
}
#[test]
fn inspect_struct() {
    println!("Task size :{:?}", std::mem::size_of::<Task>());
    println!("Frequency size :{:?}", std::mem::size_of::<Frequency>());
    println!("TaskBuilder size :{:?}", std::mem::size_of::<TaskBuilder>());
    println!("DelayTimer size :{:?}", std::mem::size_of::<DelayTimer>());
    println!(
        "ScheduleIteratorOwned size :{:?}",
        std::mem::size_of::<ScheduleIteratorOwned<Utc>>()
    );

    println!(
        "Demo Taskes size :{:?}G",
        std::mem::size_of::<Task>() * 1000000 / 1024 / 1024
    );
}
