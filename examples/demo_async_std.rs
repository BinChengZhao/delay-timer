#![allow(deprecated)]

use anyhow::Result;
use delay_timer::prelude::*;
use delay_timer::utils::convenience::functions::unblock_process_task_fn;
use smol::Timer;
use std::time::Duration;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

// You can replace the 62 line with the command you expect to execute.
#[async_std::main]
async fn main() -> Result<()> {
    // a builder for `FmtSubscriber`.
    FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // completes the builder.
        .init();

    // Build an DelayTimer that uses the default configuration of the Smol runtime internally.
    let delay_timer = DelayTimerBuilder::default()
        .smol_runtime_by_default()
        .build();

    // Develop a print job that runs in an asynchronous cycle.
    let task_instance_chain = delay_timer.insert_task(build_task_async_print()?)?;

    // Develop a php script shell-task that runs in an asynchronous cycle.
    let shell_task_instance_chain = delay_timer.insert_task(build_task_async_execute_process()?)?;

    // Get the running instance of task 1.
    let task_instance = task_instance_chain.next_with_async_wait().await?;
    Timer::after(Duration::from_secs(1)).await;

    // Cancel running task instances.
    task_instance.cancel_with_async_wait().await?;

    // Cancel running shell-task instances.
    // Probably already finished running, no need to cancel.
    let _ = shell_task_instance_chain
        .next()?
        .cancel_with_async_wait()
        .await;

    // Remove task which id is 1.
    delay_timer.remove_task(1)?;

    // No new tasks are accepted; running tasks are not affected.
    Ok(delay_timer.stop_delay_timer()?)
}

fn build_task_async_print() -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

    let body = || async {
        println!("create_async_fn_body!");

        Timer::after(Duration::from_secs(3)).await;

        println!("create_async_fn_body:i'success");
    };

    task_builder
        .set_task_id(1)
        .set_frequency_repeated_by_cron_str("* * * * * * *")
        .set_maximum_parallel_runnable_num(2)
        .spawn_async_routine(body)
}

fn build_task_async_execute_process() -> Result<Task, TaskError> {
    let task_id = 3;
    let mut task_builder = TaskBuilder::default();

    let body = move || {
        unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt".into(), task_id)
    };
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(CandyCron::Secondly))
        .set_task_id(task_id)
        .set_maximum_running_time(10)
        .set_maximum_parallel_runnable_num(1)
        .spawn_async_routine(body)
}
