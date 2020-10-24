# delayTimer
delayTimer is a task manager based on a time wheel algorithm, which makes it easy to manage timed tasks, or to periodically execute arbitrary tasks such as closures..

[![Build](https://github.com/BinChengZhao/delay_timer/workflows/Build%20and%20test/badge.svg)](
https://github.com/BinChengZhao/delay_timer/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/BinChengZhao/delay_timer)
[![Cargo](https://img.shields.io/crates/v/delay_timer.svg)](
https://crates.io/BinChengZhao/delay_timer)
[![Documentation](https://docs.rs/delay_timer/badge.svg)](
https://docs.rs/delay_timer)

## Examples


```

fn main() {
    let mut delay_timer = DelayTimer::new();
    let task_builder = TaskBuilder::default();

    delay_timer.add_task(build_task1(task_builder)).unwrap();

    delay_timer.add_task(build_task2(task_builder)).unwrap();
    delay_timer.add_task(build_task3(task_builder)).unwrap();

    sleep(Duration::new(100, 0));
    delay_timer.stop_delay_timer().unwrap();
}

fn build_task1(task_builder: TaskBuilder) -> Task {
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
    task_builder
        .set_frequency(Frequency::CountDown(2, "0/7 * * * * * *"))
        .set_task_id(7)
        .set_maximum_running_time(5)
        .spawn(body)
}

fn build_task2(task_builder: TaskBuilder) -> Task {
    let body = create_async_fn_body!({
        let mut res = surf::get("https://httpbin.org/get").await.unwrap();
        dbg!(res.body_string().await);

        Ok(())
    });
    task_builder
        .set_frequency(Frequency::CountDown(2, "0/8 * * * * * *"))
        .set_task_id(8)
        .set_maximum_running_time(5)
        .spawn(body)
}

fn build_task3(task_builder: TaskBuilder) -> Task {
    let body = unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt".into());
    task_builder
        .set_frequency(Frequency::CountDown(2, "@minutely"))
        .set_task_id(15)
        .set_maximum_running_time(5)
        .spawn(body)
}

```

There's a lot more in the [examples] directory.


## License

Licensed under either of

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


## To Do List
- [x] Disable unwrap related methods that will panic.
- [x] Thread and running task quit when delayTimer drop.
- [x] error handle need supplement.
- [x] neaten todo in code, replenish tests and benchmark.
- [x] batch-opration.
- [x] report-for-server.
- [x] TASK-TAG.
- [x] Future upgrade of delay_timer to multi-wheel mode, different excutor handling different wheels e.g. subtract laps for one wheel, run task for one wheel.

#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.


#### The author comments:

#### Make an upgrade plan for smooth updates in the future, Such as stop serve  back-up ` unfinished task`  then up new version serve load task.bak, Runing.
