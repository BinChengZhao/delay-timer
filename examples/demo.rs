use surf;
use smol::Timer;
use anyhow::Result;
use delay_timer::prelude::*;
use delay_timer::timer::timer_core::get_timestamp;
use std::thread::{current, park, sleep, Thread};
use std::time::Duration;

fn main() {
    let delay_timer = DelayTimer::new();
    let task_builder = TaskBuilder::default();

    delay_timer.add_task(build_task1(task_builder)).unwrap();

    delay_timer.add_task(build_task2(task_builder)).unwrap();
    delay_timer.add_task(build_task3(task_builder)).unwrap();
    delay_timer.add_task(build_task5(task_builder)).unwrap();

    sleep(Duration::new(2, 1_000_000));
    delay_timer.remove_task(1).unwrap();

    // cancel_task cancel_task is currently available,
    // but the user does not currently have access to reasonable input data (record_id).

    // Development of a version of delay_timer that supports rustc-stable,
    // with full canca support is expected to be completed in version 0.2.0.

    delay_timer.add_task(build_wake_task(task_builder)).unwrap();
    park();
    delay_timer.stop_delay_timer().unwrap();
}

fn build_task1(mut task_builder: TaskBuilder) -> Task {
    let body = create_async_fn_body!({
        println!("create_async_fn_body!--7");

        Timer::after(Duration::from_secs(3)).await;

        println!("create_async_fn_body:i'success");
    });
    task_builder
        .set_task_id(1)
        .set_frequency(Frequency::Repeated("0/7 * * * * * *"))
        .spawn(body)
        .unwrap()
}

fn build_task2(mut task_builder: TaskBuilder) -> Task {
    let body = create_async_fn_body!({
        let mut res = surf::get("https://httpbin.org/get").await.unwrap();
        dbg!(res.body_string().await.unwrap());
    });
    task_builder
        .set_frequency(Frequency::CountDown(2, "0/8 * * * * * *"))
        .set_task_id(2)
        .set_maximum_running_time(5)
        .spawn(body)
        .unwrap()
}

fn build_task3(mut task_builder: TaskBuilder) -> Task {
    let body = unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt".into());
    task_builder
        .set_frequency(Frequency::Once("@minutely"))
        .set_task_id(3)
        .set_maximum_running_time(5)
        .spawn(body)
        .unwrap()
}

fn build_task5(mut task_builder: TaskBuilder) -> Task {
    let body = generate_closure_template("delay_timer is easy to use. .".into());
    task_builder
        .set_frequency(Frequency::Repeated(
            "0,10,15,25,50 0/1 * * Jan-Dec * 2020-2100",
        ))
        .set_task_id(5)
        .set_maximum_running_time(5)
        .spawn(body)
        .unwrap()
}

pub fn generate_closure_template(
    name: String,
) -> impl Fn(TaskContext) -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
    move |context| {
        let future_inner = async_template(get_timestamp() as i32, name.clone());

        let future = async move {
            future_inner.await.unwrap();
            context.finishe_task().await;
        };
        create_delay_task_handler(async_spawn(future))
    }
}

pub async fn async_template(id: i32, name: String) -> Result<()> {
    let url = format!("https://httpbin.org/get?id={}&name={}", id, name);
    let mut res = surf::get(url).await.unwrap();
    dbg!(res.body_string().await.unwrap());

    Ok(())
}

fn build_wake_task(mut task_builder: TaskBuilder) -> Task {
    let thread: Thread = current();
    let body = move |_| {
        println!("bye bye");
        thread.unpark();
        create_default_delay_task_handler()
    };

    //TODO:use candy.
    task_builder
        .set_frequency(Frequency::Once("@minutely"))
        .set_task_id(700)
        .set_maximum_running_time(50)
        .spawn(body)
        .unwrap()
}
