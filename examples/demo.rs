use delay_timer::{
    create_async_fn_body,
    delay_timer::DelayTimer,
    timer::task::{Frequency, Task, TaskBuilder},
    utils::functions::{create_delay_task_handler, unblock_process_task_fn},
};
use smol::Timer;
use std::thread::sleep;
use std::time::Duration;
use surf;

use delay_timer::timer::timer_core::get_timestamp;
use delay_timer::{async_spawn, DelayTaskHandler};

use anyhow::Result;

fn main() {
    let mut delay_timer = DelayTimer::new();
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

    sleep(Duration::new(90, 0));
    delay_timer.stop_delay_timer().unwrap();
}

fn build_task1(mut task_builder: TaskBuilder) -> Task {
    let body = create_async_fn_body!({
        println!("create_async_fn_body!--7");

        Timer::after(Duration::from_secs(3)).await;

        println!("create_async_fn_body:i'success");
        Ok(())
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

        Ok(())
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
) -> impl Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
    move || {
        create_delay_task_handler(async_spawn(async_template(
            get_timestamp() as i32,
            name.clone(),
        )))
    }
}

pub async fn async_template(id: i32, name: String) -> Result<()> {
    let url = format!("https://httpbin.org/get?id={}&name={}", id, name);
    let mut res = surf::get(url).await.unwrap();
    dbg!(res.body_string().await.unwrap());

    Ok(())
}
