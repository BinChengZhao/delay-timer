use delay_timer::{
    create_async_fn_body,
    delay_timer::DelayTimer,
    timer::task::{Frequency, TaskBuilder},
    utils::functions::create_delay_task_handler,
};
use smol::Timer;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut delay_timer = DelayTimer::new();
    let mut task_builder = TaskBuilder::default();
    let body = create_async_fn_body!({
        println!("create_async_fn_body!--7");
        Timer::after(Duration::from_secs(2)).await;

        println!("create_async_fn_body:i'm part of success--1");

        Timer::after(Duration::from_secs(2)).await;

        println!("create_async_fn_body:i'm part of success--2");

        Timer::after(Duration::from_secs(3)).await;

        println!("create_async_fn_body:i'success");
        Ok(())
    });
    task_builder.set_frequency(Frequency::CountDown(2, "0/7 * * * * * *"));
    task_builder.set_task_id(7);
    task_builder.set_maximum_running_time(5);
    let _task = task_builder.spawn(body);
    delay_timer.add_task(_task).unwrap();

    sleep(Duration::new(100, 0));
    delay_timer.stop_delay_timer().unwrap();
}
