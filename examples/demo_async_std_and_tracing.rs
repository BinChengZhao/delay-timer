#![allow(deprecated)]

use anyhow::Result;
use delay_timer::prelude::*;
use delay_timer::utils::convenience::functions::unblock_process_task_fn;
use smol::Timer;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[async_std::main]
async fn main() -> Result<()> {
    // a builder for `FmtSubscriber`.
    FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // completes the builder.
        .init();

    let delay_timer = DelayTimerBuilder::default()
        .smol_runtime_by_default()
        .build();
    for i in 0..1000 {
        delay_timer.add_task(build_task_async_execute_process(i)?)?;
    }

    info!("==== All job is be init! ====");
    for _ in 0..300 {
        Timer::after(Duration::from_secs(60)).await;
    }
    Ok(delay_timer.stop_delay_timer()?)
}

fn build_task_async_execute_process(task_id: u64) -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

    // Remind the user to set a timeout that must be reasonable.
    let body = move || unblock_process_task_fn("echo hello".into(), task_id);
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(CandyCron::Secondly))
        .set_task_id(task_id)
        .set_maximum_running_time(2)
        .set_maximum_parallel_runnable_num(1)
        .spawn_async_routine(body)
}
