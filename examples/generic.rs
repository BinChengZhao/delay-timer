#![allow(deprecated)]

use anyhow::Result;
use delay_timer::prelude::*;
use smol::Timer;
use std::any::{type_name, Any};
use std::thread::park_timeout;
use std::time::Duration;
use surf;

// cargo run --package delay_timer --example generic --features=full

fn main() -> Result<()> {
    let delay_timer = DelayTimerBuilder::default().enable_status_report().build();

    // Develop tasks with generic parameters. that runs in an asynchronous cycle.
    delay_timer.add_task(build_generic_task_async_request(Dog)?)?;

    // Give a little time to observe the output.
    park_timeout(Duration::from_secs(30));

    // No new tasks are accepted; running tasks are not affected.
    delay_timer.stop_delay_timer()?;

    Ok(())
}

fn build_generic_task_async_request<T: Animal>(animal: T) -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

    let other_animal = Cat;
    let int_animal = 1;

    let body = move || {
        let animal_ref = animal.clone();
        let other_animal_ref = other_animal.clone();
        async move {
            if let Ok(mut res) = surf::get("https://httpbin.org/get").await {
                dbg!(res.body_string().await.unwrap_or_default());
                animal_ref.call();
                other_animal_ref.call();
                <i32 as Animal>::call(&int_animal);

                Timer::after(Duration::from_secs(3)).await;
                dbg!("Task2 is done.");
            }
        }
    };

    task_builder
        .set_frequency_count_down_by_seconds(1, 15)
        .set_task_id(2)
        .set_maximum_running_time(5)
        .spawn_async_routine(body)
}

trait ThreadSafe: Any + Sized + Clone + Send + Sync + 'static {}
impl<T: Any + Sized + Clone + Send + Sync + 'static> ThreadSafe for T {}

trait Animal: ThreadSafe {
    fn call(&self);
}

impl<T: ThreadSafe> Animal for T {
    fn call(&self) {
        println!("this is {}", type_name::<T>());
    }
}

#[derive(Clone, Copy)]
struct Dog;

#[derive(Clone, Copy)]
struct Cat;
