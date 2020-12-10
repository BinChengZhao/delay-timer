#![feature(ptr_internals)]
use delay_timer::prelude::*;
use std::{
    str::FromStr,
    sync::{
        atomic::{
            AtomicUsize,
            Ordering::{Acquire, Release},
        },
        Arc,
    },
    thread::{self, park_timeout},
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
        .spawn(body)
        .unwrap();
    delay_timer.add_task(task).unwrap();

    let mut i = 0;

    loop {
        //Testing, whether the mission is performing as expected.
        assert_eq!(i, share_num.load(Acquire));

        i = i + 1;
        park_timeout(Duration::from_micros(6_100_000));

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
        .spawn(body)
        .unwrap();
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
    use tokio::runtime::Runtime;

    println!("Task size :{:?}", std::mem::size_of::<Task>());
    println!("Frequency size :{:?}", std::mem::size_of::<Frequency>());
    println!("TaskBuilder size :{:?}", std::mem::size_of::<TaskBuilder>());
    println!("DelayTimer size :{:?}", std::mem::size_of::<DelayTimer>());
    println!("Runtime size :{:?}", std::mem::size_of::<Runtime>());

    println!(
        "ScheduleIteratorOwned size :{:?}",
        std::mem::size_of::<ScheduleIteratorOwned<Utc>>()
    );

    let mut s = Schedule::from_str("* * * * * * *")
        .unwrap()
        .upcoming_owned(Utc);

    let mut s1 = s.clone();

    println!("{:?}, {:?}", s.next(), s1.next());
    thread::sleep(Duration::from_secs(1));
    println!("{:?}, {:?}", s.next(), s1.next());
    let mut s2 = s1.clone();
    thread::sleep(Duration::from_secs(1));
    println!("{:?}, {:?}", s.next(), s2.next());
}
