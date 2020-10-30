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

fn main() {
    let mut delay_timer = DelayTimer::new();
    let task_builder = TaskBuilder::default();

    delay_timer.add_task(build_task1(task_builder)).unwrap();

    delay_timer.add_task(build_task2(task_builder)).unwrap();
    delay_timer.add_task(build_task3(task_builder)).unwrap();

    sleep(Duration::new(100, 0));
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
        .set_frequency(Frequency::Repeated("0/7 * * * * * *"))
        .spawn(body)
}

fn build_task2(mut task_builder: TaskBuilder) -> Task {
    let body = create_async_fn_body!({
        let mut res = surf::get("https://httpbin.org/get").await.unwrap();
        dbg!(res.body_string().await.unwrap());

        Ok(())
    });
    task_builder
        .set_frequency(Frequency::CountDown(2, "0/8 * * * * * *"))
        .set_task_id(8)
        .set_maximum_running_time(5)
        .spawn(body)
}

fn build_task3(mut task_builder: TaskBuilder) -> Task {
    let body = unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt".into());
    task_builder
        .set_frequency(Frequency::Once("@minutely"))
        .set_task_id(15)
        .set_maximum_running_time(5)
        .spawn(body)
}
