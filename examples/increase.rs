#![allow(deprecated)]

use surf;

use delay_timer::prelude::*;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use std::thread::{current, park, Thread};

#[derive(Debug, Clone, Copy)]
struct SafePointer(NonNull<Arc<AtomicUsize>>);

unsafe impl Send for SafePointer {}
unsafe impl Sync for SafePointer {}
impl Deref for SafePointer {
    type Target = NonNull<Arc<AtomicUsize>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Remember close terminal can speed up because of
// printnl! block process if stand-pipe if full.
fn main() -> AnyResult<()> {
    let delay_timer = DelayTimer::new();

    // The Sync-Task run_flay.
    let mut run_flag = Arc::new(AtomicUsize::new(0));
    // cross thread share raw-pointer.
    let run_flag_ref: SafePointer = SafePointer(
        NonNull::new(&mut run_flag as *mut Arc<AtomicUsize>)
            .ok_or_else(|| anyhow!("Can't init NonNull."))?,
    );

    // Sync-Task body.
    let body = get_increase_fn(run_flag_ref);
    // Waker-Task body.
    let end_body = get_wake_fn(current(), run_flag_ref);
    // Async-Task body.
    let async_body = get_async_fn;

    let mut task_builder = TaskBuilder::default();

    // The common task attr.
    task_builder
        .set_frequency_once_by_seconds(30)
        .set_maximum_running_time(90);

    for i in 0..1000 {
        let task = task_builder.set_task_id(i).spawn_routine(body)?;
        delay_timer.add_task(task)?;
    }

    task_builder.set_frequency_count_down_by_seconds(58, 1);
    for i in 1000..1300 {
        let task = task_builder
            .set_task_id(i)
            .spawn_async_routine(async_body)?;
        delay_timer.add_task(task)?;
    }

    let task = task_builder
        .set_task_id(8888)
        .set_frequency_once_by_minutes(1)
        .spawn_routine(end_body)?;
    delay_timer.add_task(task)?;

    park();
    Ok(())
}

fn get_increase_fn(run_flag_ref: SafePointer) -> impl Copy + Fn() {
    move || {
        let local_run_flag = run_flag_ref.as_ptr();

        unsafe {
            (*local_run_flag).fetch_add(1, SeqCst);
        }
    }
}

fn get_wake_fn(
    thread: Thread,
    run_flag_ref: SafePointer,
) -> impl Fn() -> () + Clone + Send + Sync + 'static {
    move || {
        let local_run_flag = run_flag_ref.as_ptr();
        unsafe {
            println!(
                "end time {}, result {}",
                timestamp(),
                (*local_run_flag).load(SeqCst)
            );
        }
        thread.unpark();
    }
}

fn get_async_fn() -> impl std::future::Future {
    async {
        if let Ok(mut res) = surf::get("https://httpbin.org/get").await {
            let body_str = res.body_string().await.unwrap_or_default();
            println!("{}", body_str);
        }
    }
}
