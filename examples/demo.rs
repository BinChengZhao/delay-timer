use anyhow::Result;
use delay_timer::prelude::*;
#[allow(deprecated)]
use delay_timer::utils::convenience::functions::unblock_process_task_fn;
use smol::Timer;
use std::thread::{current, park, Thread};
use std::time::Duration;
use surf;

// cargo run --package delay_timer --example demo --features=full

fn main() -> Result<()> {
    let delay_timer = DelayTimerBuilder::default().enable_status_report().build();

    // Develop a print job that runs in an asynchronous cycle.
    let task_instance_chain = delay_timer.insert_task(build_task_async_print()?)?;

    // Develop an http request task that runs in an asynchronous cycle.
    delay_timer.add_task(build_task_async_request()?)?;

    // Develop a php script task that runs in an asynchronous cycle.
    delay_timer.add_task(build_task_async_execute_process()?)?;

    // Develop a task that runs in an asynchronous cycle (using a custom asynchronous template).
    delay_timer.add_task(build_task_customized_async_task()?)?;

    // Get the running instance of task 1.
    let task_instance = task_instance_chain.next_with_wait()?;

    // Cancel running task instances.
    task_instance.cancel_with_wait()?;

    // Remove task which id is 1.
    delay_timer.remove_task(1)?;

    // Develop a task that runs in an asynchronous cycle to wake up the current thread.
    delay_timer.add_task(build_wake_task()?)?;

    park();

    // No new tasks are accepted; running tasks are not affected.
    delay_timer.stop_delay_timer()?;

    Ok(())
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
        .set_frequency_repeated_by_cron_str("@secondly")
        .set_maximum_parallel_runnable_num(2)
        .spawn_async_routine(body)
}

fn build_task_async_request() -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

    let body = || async {
        if let Ok(mut res) = surf::get("https://httpbin.org/get").await {
            dbg!(res.body_string().await.unwrap_or_default());

            Timer::after(Duration::from_secs(3)).await;
            dbg!("Task2 is done.");
        }
    };

    task_builder
        .set_frequency_repeated_by_seconds(8)
        .set_task_id(2)
        .set_maximum_running_time(5)
        .spawn_async_routine(body)
}

fn build_task_async_execute_process() -> Result<Task, TaskError> {
    let task_id = 3;
    let mut task_builder = TaskBuilder::default();

    let body = move || {
        #[allow(deprecated)]
        unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt".into(), task_id)
    };
    task_builder
        .set_frequency_repeated_by_minutes(1)
        .set_task_id(task_id)
        .set_maximum_running_time(5)
        .spawn_async_routine(body)
}

fn build_task_customized_async_task() -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

    let name = String::from("build_task_customized_async_task");
    let body = move || {
        let name = name.clone();
        async move {
            async_template(timestamp() as i32, name).await;
        }
    };
    #[allow(deprecated)]
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::LoveTime))
        .set_task_id(5)
        .set_maximum_running_time(5)
        .spawn_async_routine(body)
}

pub async fn async_template(id: i32, name: String) {
    let url = format!("https://httpbin.org/get?id={}&name={}", id, name);
    if let Ok(mut res) = surf::get(url).await {
        dbg!(res.body_string().await.unwrap_or_default());
    }
}

fn build_wake_task() -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

    let thread: Thread = current();
    let body = move || {
        println!("bye bye");
        thread.unpark();
    };

    task_builder
        .set_frequency_repeated_by_cron_str("@minutely")
        .set_task_id(700)
        .set_maximum_running_time(50)
        .spawn_routine(body)
}

// Custom cron-expression syntax sugar mapping.
#[allow(dead_code)]
enum AuspiciousTime {
    PerSevenSeconds,
    PerEightSeconds,
    LoveTime,
    PerDayFiveAclock,
}

impl Into<CandyCronStr> for AuspiciousTime {
    fn into(self) -> CandyCronStr {
        match self {
            Self::PerSevenSeconds => CandyCronStr("0/7 * * * * * *".to_string()),
            Self::PerEightSeconds => CandyCronStr("0/8 * * * * * *".to_string()),
            Self::LoveTime => CandyCronStr("0,10,15,25,50 0/1 * * Jan-Dec * 2020-2100".to_string()),
            Self::PerDayFiveAclock => CandyCronStr("01 00 1 * * * *".to_string()),
        }
    }
}
