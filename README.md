# delay-timer
Time-manager of delayed tasks. Like crontab, but synchronous asynchronous tasks are possible, and dynamic add/cancel/remove is supported.

delay-timer is a task manager based on a time wheel algorithm, which makes it easy to manage timed tasks, or to periodically execute arbitrary tasks such as closures.

The underlying runtime is based on the optional smol and tokio, and you can build your application with either one.

Since the library currently includes features such as #[bench], it needs to be developed in a nightly version.

Except for the simple execution in a few seconds, you can also specify a specific date, 
such as Sunday at 4am to execute a backup task.

Supports configuration of the maximum number of parallelism of tasks.

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


``` rust
//TODO: Cases should be written short and beautiful .
fn main() {
    let delay_timer = DelayTimerBuilder::default().enable_status_report().build();
    let task_builder = TaskBuilder::default();

    delay_timer.add_task(build_task1(task_builder)).unwrap();
    delay_timer.add_task(build_task3(task_builder)).unwrap();
    delay_timer.add_task(build_task5(task_builder)).unwrap();

    let task1_record_id = filter_task_recodeid(&delay_timer, |&x| x.get_task_id() == 1).unwrap();
    delay_timer.cancel_task(1, task1_record_id);
    delay_timer.remove_task(1).unwrap();

    delay_timer.add_task(build_wake_task(task_builder)).unwrap();
    park();
    delay_timer.stop_delay_timer().unwrap();
}

fn build_task1(mut task_builder: TaskBuilder) -> Task {
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


fn build_task3(mut task_builder: TaskBuilder) -> Task {
    let body = unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt".into());
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(CandyCron::Minutely))
        .set_task_id(3)
        .set_maximum_running_time(5)
        .spawn(body)
        .unwrap()
}

fn build_task5(mut task_builder: TaskBuilder) -> Task {
    let body = generate_closure_template("delay_timer is easy to use. .".into());
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::LoveTime))
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

    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(CandyCron::Minutely))
        .set_task_id(700)
        .set_maximum_running_time(50)
        .spawn(body)
        .unwrap()
}
```

There's a lot more in the [examples] directory.


## License

Licensed under either of

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)


## To Do List
- [x] Support async-local-executor API and defalut start.
- [√] Support tokio Ecology.
- [x] Disable unwrap related methods that will panic.
- [x] Thread and running task quit when delayTimer drop.
- [x] error handle need supplement.
- [x] neaten todo in code, replenish tests and benchmark.
- [x] batch-opration.
- [√] report-for-server.
- [x] TASK-TAG.
- [x] Future upgrade of delay_timer to multi-wheel mode, different excutor handling different wheels e.g. subtract laps for one wheel, run task for one wheel.

#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.


#### The author comments:

#### Make an upgrade plan for smooth updates in the future, Such as stop serve  back-up ` unfinished task`  then up new version serve load task.bak, Runing.
