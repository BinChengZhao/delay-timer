use anyhow::Result;
use delay_timer::prelude::*;
#[allow(deprecated)]
use delay_timer::utils::convenience::functions::unblock_process_task_fn;
use hyper::{Client, Uri};
use std::time::Duration;
use tokio::time::sleep;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

// You can replace the 66 line with the command you expect to execute.
#[tokio::main]
async fn main() -> Result<()> {
    // a builder for `FmtSubscriber`.
    FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // completes the builder.
        .init();

    // In addition to the mixed (smol & tokio) runtime
    // You can also share a tokio runtime with delayTimer, please see api `DelayTimerBuilder::tokio_runtime` for details.

    // Build an DelayTimer that uses the default configuration of the Smol runtime internally.
    let delay_timer = DelayTimerBuilder::default().build();

    // Develop a print job that runs in an asynchronous cycle.
    let task_instance_chain = delay_timer.insert_task(build_task_async_print()?)?;

    // Develop a php script shell-task that runs in an asynchronous cycle.
    let shell_task_instance_chain = delay_timer.insert_task(build_task_async_execute_process()?)?;

    // Get the running instance of task 1.
    let task_instance = task_instance_chain.next_with_async_wait().await?;

    // Wating request done then cancel it (It's just a custom logic).
    sleep(Duration::from_secs(1)).await;

    // Cancel running task instances.
    task_instance.cancel_with_async_wait().await?;

    // Cancel running shell-task instances.
    // Probably already finished running, no need to cancel.
    let _ = shell_task_instance_chain
        .next_with_async_wait()
        .await?
        .cancel_with_async_wait()
        .await?;

    // Remove task which id is 1.
    delay_timer.remove_task(1)?;

    // No new tasks are accepted; running tasks are not affected.
    Ok(delay_timer.stop_delay_timer()?)
}

fn build_task_async_print() -> Result<Task, TaskError> {
    let id = 1;
    let name = String::from("someting");
    let mut task_builder = TaskBuilder::default();

    let body = move || {
        let name_ref = name.clone();
        async move {
            async_template(id, name_ref).await.expect("Request failed.");

            sleep(Duration::from_secs(3)).await;

            println!("create_async_fn_body:i'success");
        }
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
        #[allow(deprecated)]
        unblock_process_task_fn("/opt/homebrew/bin/php /Users/bincheng_paopao/project/repo/rust/myself/delay-timer/examples/try_spawn.php >> ./try_spawn.txt".into(), task_id)
    };
    task_builder
        .set_frequency_repeated_by_seconds(1)
        .set_task_id(task_id)
        .set_maximum_running_time(10)
        .set_maximum_parallel_runnable_num(1)
        .spawn_async_routine(body)
}

pub async fn async_template(id: i32, name: String) -> Result<()> {
    let client = Client::new();

    // The default connector does not handle TLS.
    // Speaking to https destinations will require configuring a connector that implements TLS.
    // So use http for test.
    let url = format!("http://httpbin.org/get?id={}&name={}", id, name);
    let uri: Uri = url.parse()?;

    let res = client.get(uri).await?;
    println!("Response: {}", res.status());
    // Concatenate the body stream into a single buffer...
    let buf = hyper::body::to_bytes(res).await?;
    println!("body: {:?}", buf);
    Ok(())
}
