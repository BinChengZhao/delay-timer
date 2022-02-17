#![allow(deprecated)]

use anyhow::Result;
use delay_timer::prelude::*;
use hyper::{Client, Uri};
use std::thread::{current, park, Thread};

// When you try to run that's example nedd add feature `tokio-support`.
// cargo run --example=cycle_tokio_task --features=tokio-support

fn main() -> Result<()> {
    // Build an DelayTimer that uses the default configuration of the Tokio runtime internally.
    let delay_timer = DelayTimerBuilder::default()
        .tokio_runtime_by_default()
        .build();

    // Develop a task that runs in an asynchronous cycle (using a custom asynchronous template).
    delay_timer.add_task(build_task_customized_async_task()?)?;

    // Develop a task that runs in an asynchronous cycle to wake up the current thread.
    delay_timer.add_task(build_wake_task()?)?;
    park();

    // No new tasks are accepted; running tasks are not affected.
    delay_timer.stop_delay_timer()?;

    Ok(())
}

fn build_task_customized_async_task() -> Result<Task> {
    let mut task_builder = TaskBuilder::default();

    let body = generate_closure_template;

    Ok(task_builder
        .set_frequency_repeated_by_cron_str("10,15,25,50 0/1 * * Jan-Dec * 2020-2100")
        .set_task_id(5)
        .set_maximum_running_time(15)
        .spawn_async_routine(body)?)
}

fn build_wake_task() -> Result<Task> {
    let mut task_builder = TaskBuilder::default();

    let thread: Thread = current();
    let body = move || {
        println!("bye bye");
        thread.unpark();
    };

    Ok(task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousDay::Wake))
        .set_task_id(7)
        .set_maximum_running_time(50)
        .spawn_routine(body)?)
}

pub async fn generate_closure_template() {
    let name: String = "'delay_timer-is-easy-to-use.'".into();
    let future_inner = async_template(timestamp() as i32, name.clone());
    future_inner.await.ok();
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

enum AuspiciousDay {
    Wake,
}

impl Into<CandyCronStr> for AuspiciousDay {
    fn into(self) -> CandyCronStr {
        match self {
            Self::Wake => CandyCronStr("0 * * * Jan-Dec * 2020-2100".to_string()),
        }
    }
}
