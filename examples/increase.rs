#![feature(ptr_internals)]
use delay_timer::{prelude::*};

use std::{
    ptr::Unique,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    thread::{current, park, Thread},
};
use surf;
//TODO:Remember close terminal can speed up because of
//printnl! block process if stand-pipe if full.
fn main() {
    let delay_timer = DelayTimer::new();
    let mut run_flag = Arc::new(AtomicUsize::new(0));
    let run_flag_ref: Option<Unique<Arc<AtomicUsize>>> = Unique::new(&mut run_flag);

    let body = get_increase_fn(run_flag_ref);
    let end_body = get_end_fn(current(), run_flag_ref);
    let async_body = get_async_fn();

    let mut task_builder = TaskBuilder::default();

    task_builder
        .set_frequency(Frequency::CountDown(1, "30 * * * * * *"))
        .set_maximum_running_time(90);

    for i in 0..1000 {
        let task = task_builder.set_task_id(i).spawn(body).unwrap();
        delay_timer.add_task(task).unwrap();
    }

    task_builder.set_frequency(Frequency::CountDown(1, "58 * * * * * *"));
    for i in 1000..1300 {
        let task = task_builder.set_task_id(i).spawn(async_body).unwrap();
        delay_timer.add_task(task).unwrap();
    }

    let task = task_builder
        .set_task_id(8888)
        .set_frequency(Frequency::CountDown(1, "@minutely"))
        .spawn(end_body)
        .unwrap();
    delay_timer.add_task(task).unwrap();

    park();
}

fn get_increase_fn(
    run_flag_ref: Option<Unique<Arc<AtomicUsize>>>,
) -> impl Copy + Fn() -> Box<dyn DelayTaskHandler> {
    move || {
        let local_run_flag = run_flag_ref.unwrap().as_ptr();

        unsafe {
            (*local_run_flag).fetch_add(1, SeqCst);
        }
        create_default_delay_task_handler()
    }
}

fn get_end_fn(
    thread: Thread,
    run_flag_ref: Option<Unique<Arc<AtomicUsize>>>,
) -> impl Fn() -> Box<dyn DelayTaskHandler> {
    move || {
        let local_run_flag = run_flag_ref.unwrap().as_ptr();
        unsafe {
            println!(
                "end time {}, result {}",
                get_timestamp(),
                (*local_run_flag).load(SeqCst)
            );
        }
        thread.unpark();
        create_default_delay_task_handler()
    }
}

fn get_async_fn() -> impl Copy + Fn() -> Box<dyn DelayTaskHandler> {
    create_async_fn_body!({
        let mut res = surf::get("https://httpbin.org/get").await.unwrap();
        let body_str = res.body_string().await.unwrap();
        println!("{}", body_str);
        Ok(())
    })
}
