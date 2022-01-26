#![allow(deprecated)]

use anyhow::Result;
use delay_timer::prelude::*;
use delay_timer::utils::convenience::functions::tokio_unblock_process_task_fn;
use smol::Timer;
use tokio::runtime::Runtime;

use std::sync::Arc;
use std::time::Duration;
// cargo run --package delay_timer --example dynamic_cancel --features=full

fn main() -> Result<()> {
    // Dynamically cancel in the context of `synchronization`.
    sync_cancel()?;

    println!("");
    // Dynamic cancellation in `asynchronous` contexts.
    async_cancel()
}

/// In Synchronization Context.
fn sync_cancel() -> Result<()> {
    // Build an DelayTimer that uses the default configuration of the Smol runtime internally.
    let delay_timer = DelayTimerBuilder::default().build();

    // A chain of task instances.
    let instance_chain = delay_timer.insert_task(build_task_async_print()?)?;

    // Get the next task instance and cancel it immediately after getting it.
    instance_chain.next_with_wait()?.cancel_with_wait()?;

    Ok(())
}

/// In Asynchronous Context.
fn async_cancel() -> Result<()> {
    // Customize a tokio runtime.
    let tokio_rt = Arc::new(Runtime::new()?);

    // Build an DelayTimer that uses the Customize a tokio runtime.
    let delay_timer = DelayTimerBuilder::default()
        .tokio_runtime_shared_by_custom(tokio_rt.clone())
        .build();

    tokio_rt.block_on(async {
        // A chain of print-task instances.
        let task_instance_chain = delay_timer.insert_task(build_task_async_print()?)?;

        // A chain of shell-task instances.
        let shell_task_instance_chain =
            delay_timer.insert_task(build_task_async_execute_process()?)?;

        // Get the next print-task instance and cancel it immediately after getting it.
        let cancel_print_task_instance = async {
            task_instance_chain
                .next_with_async_wait()
                .await?
                .cancel_with_async_wait()
                .await
        };
        cancel_print_task_instance.await?;

        // Get the next shell-task instance and cancel it immediately after getting it.
        let cancel_shell_task_instance = async {
            shell_task_instance_chain
                .next_with_async_wait()
                .await?
                .cancel_with_async_wait()
                .await
        };
        cancel_shell_task_instance.await?;

        Ok(())
    })
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
        .set_frequency_repeated_by_seconds(6)
        .set_maximum_parallel_runnable_num(2)
        .spawn_async_routine(body)
}

fn build_task_async_execute_process() -> Result<Task, TaskError> {
    let task_id = 3;
    let mut task_builder = TaskBuilder::default();

    let body = move || {
        tokio_unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn_async_routine.php >> ./try_spawn_async_routine.txt".into(), task_id)
    };
    task_builder
        .set_frequency_repeated_by_seconds(1)
        .set_task_id(task_id)
        .set_maximum_running_time(10)
        .set_maximum_parallel_runnable_num(1)
        .spawn_async_routine(body)
}
