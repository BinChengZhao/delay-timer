# delay-timer
delay-timer is a task manager based on a time wheel algorithm, which makes it easy to manage timed tasks, or to periodically execute arbitrary tasks such as closures..

The underlying runtime is currently based on smol, so upper level applications that want to extend asynchronous functionality need to use libraries that are compatible with smol.

Since the library currently includes features such as #[bench], it needs to be developed in a nightly version.

[![Build](https://github.com/BinChengZhao/delay-timer/workflows/Build%20and%20test/badge.svg)](
https://github.com/BinChengZhao/delay-timer/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/BinChengZhao/delay-timer)
[![Cargo](https://img.shields.io/crates/v/delay_timer.svg)](
https://crates.io/BinChengZhao/delay_timer)
[![Documentation](https://docs.rs/delay_timer/badge.svg)](
https://docs.rs/delay_timer)
![image](https://github.com/BinChengZhao/delay-timer/blob/master/structural_drawing/DelayTImer.png)
## Examples


```

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
}

fn build_task3(mut task_builder: TaskBuilder) -> Task {
    let body = unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt".into());
    task_builder
        .set_frequency(Frequency::Once("@minutely"))
        .set_task_id(3)
        .set_maximum_running_time(5)
        .spawn(body)
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


```

There's a lot more in the [examples] directory.


## License

Licensed under either of

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)


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
