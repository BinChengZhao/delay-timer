use delay_timer::prelude::*;

use anyhow::Result;

use smol::Timer;
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<()> {
    // Build an DelayTimer that uses the default configuration of the Smol runtime internally.
    let delay_timer = DelayTimerBuilder::default().build();

    // Develop a print job that runs in an asynchronous cycle.
    let task_instance_chain = delay_timer.insert_task(build_task_async_print())?;

    // Get the running instance of task 1.
    let task_instance = task_instance_chain.next_with_async_wait().await?;

    // Cancel running task instances.
    task_instance.cancel_with_async_wait().await?;

    // Remove task which id is 1.
    delay_timer.remove_task(1)?;

    // FIXME: try cause error. 
    Timer::after(Duration::from_secs(3)).await;
    let task_instance = task_instance_chain.next_with_async_wait().await?;


    // No new tasks are accepted; running tasks are not affected.
    delay_timer.stop_delay_timer()
}

fn build_task_async_print() -> Task {
    let mut task_builder = TaskBuilder::default();

    let body = create_async_fn_body!({
        println!("create_async_fn_body!");

        Timer::after(Duration::from_secs(3)).await;

        println!("create_async_fn_body:i'success");
    });

    task_builder
        .set_task_id(1)
        .set_frequency_by_candy(CandyFrequency::Repeated(CandyCron::Secondly))
        .set_maximun_parallel_runable_num(2)
        .spawn(body)
        .unwrap()
}
