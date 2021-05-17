# delay-timer  
[![Build](https://github.com/BinChengZhao/delay-timer/workflows/Build%20and%20test/badge.svg)](
https://github.com/BinChengZhao/delay-timer/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/BinChengZhao/delay-timer)
[![Cargo](https://img.shields.io/crates/v/delay_timer.svg)](
https://crates.io/crates/delay_timer)
[![Documentation](https://docs.rs/delay_timer/badge.svg)](
https://docs.rs/delay_timer)

Time-manager of delayed tasks. Like crontab, but synchronous asynchronous tasks are possible, and dynamic add/cancel/remove is supported.

delay-timer is a task manager based on a time wheel algorithm, which makes it easy to manage timed tasks, or to periodically execute arbitrary tasks such as closures.

The underlying runtime is based on the optional smol and tokio, and you can build your application with either one.

The minimum-supported version of `rustc` is **1.49**.

Except for the simple execution in a few seconds, you can also specify a specific date, 
such as Sunday at 4am to execute a backup task.

##### Supports configuration of the maximum number of parallelism of tasks.
##### Dynamically cancel a running task instance by means of a handle.

![image](https://github.com/BinChengZhao/delay-timer/blob/master/structural_drawing/DelayTImer.png)


## Examples


 ```rust

use anyhow::Result;
use delay_timer::prelude::*;

fn main() -> Result<()> {
    // Build an DelayTimer that uses the default configuration of the Smol runtime internally.
    let delay_timer = DelayTimerBuilder::default().build();

    // Develop a print job that runs in an asynchronous cycle.
    // A chain of task instances.
    let task_instance_chain = delay_timer.insert_task(build_task_async_print()?)?;

    // Get the running instance of task 1.
    let task_instance = task_instance_chain.next_with_wait()?;

    // Cancel running task instances.
    task_instance.cancel_with_wait()?;

    // Remove task which id is 1.
    delay_timer.remove_task(1)?;

    // No new tasks are accepted; running tasks are not affected.
    delay_timer.stop_delay_timer()?;

    Ok(())
}

fn build_task_async_print() -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

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
}

 ```

Use in asynchronous contexts.
 ``` rust

use delay_timer::prelude::*;

use anyhow::Result;

use smol::Timer;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // In addition to the mixed (smol & tokio) runtime
    // You can also share a tokio runtime with delayTimer, please see api `DelayTimerBuilder::tokio_runtime` for details.

    // Build an DelayTimer that uses the default configuration of the Smol runtime internally.
    let delay_timer = DelayTimerBuilder::default().build();

    // Develop a print job that runs in an asynchronous cycle.
    let task_instance_chain = delay_timer.insert_task(build_task_async_print()?)?;

    // Get the running instance of task 1.
    let task_instance = task_instance_chain.next_with_async_wait().await?;

    // Cancel running task instances.
    task_instance.cancel_with_async_wait().await?;


    // Remove task which id is 1.
    delay_timer.remove_task(1)?;

    // No new tasks are accepted; running tasks are not affected.
    delay_timer.stop_delay_timer()
}

fn build_task_async_print() -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

    let body = create_async_fn_body!({
        println!("create_async_fn_body!");

        Timer::after(Duration::from_secs(3)).await;

        println!("create_async_fn_body:i'success");
    });

    task_builder
        .set_task_id(1)
        .set_frequency(Frequency::Repeated("*/6 * * * * * *"))
        .set_maximun_parallel_runable_num(2)
        .spawn(body)
}


 ```


 Capture the specified environment information and build the closure & task:

 ``` rust
 #[macro_use]
 use delay_timer::prelude::*;

 use std::sync::atomic::{
     AtomicUsize,
     Ordering::{Acquire, Release},
 };
 use std::sync::Arc;


 let delay_timer = DelayTimer::new();

 let share_num = Arc::new(AtomicUsize::new(0));
 let share_num_bunshin = share_num.clone();

 let body = create_async_fn_body!((share_num_bunshin){
     share_num_bunshin_ref.fetch_add(1, Release);
 });

 let task = TaskBuilder::default()
     .set_frequency_by_candy(CandyFrequency::CountDown(9, CandyCron::Secondly))
     .set_task_id(1)
     .set_maximun_parallel_runable_num(3)
     .spawn(body)?;

 delay_timer.add_task(task);

 ```



 Building customized-dynamic future tasks:
 ``` rust
 #[macro_use]
 use delay_timer::prelude::*;
 use hyper::{Client, Uri};

fn build_task_customized_async_task() -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

    let body = generate_closure_template("delay_timer is easy to use. .".into());
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::LoveTime))
        .set_task_id(5)
        .set_maximum_running_time(5)
        .spawn(body)
}

pub fn generate_closure_template(
    name: String,
) -> impl Fn(TaskContext) -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
    move |context| {
        let future_inner = async_template(get_timestamp() as i32, name.clone());

        let future = async move {
            future_inner.await;
            context.finishe_task().await;
        };
        create_delay_task_handler(async_spawn(future))
    }
}

pub async fn async_template(id: i32, name: String) -> Result<()> {
    let url = format!("https://httpbin.org/get?id={}&name={}", id, name);
    let mut res = surf::get(url).await?;
    dbg!(res.body_string().await?);

    Ok(())
}

  enum AuspiciousTime {
      PerSevenSeconds,
      PerEightSeconds,
      LoveTime,
  }
 
  impl Into<CandyCronStr> for AuspiciousTime {
      fn into(self) -> CandyCronStr {
          match self {
              Self::PerSevenSeconds => CandyCronStr("0/7 * * * * * *".to_string()),
              Self::PerEightSeconds => CandyCronStr("0/8 * * * * * *".to_string()),
              Self::LoveTime => CandyCronStr("0,10,15,25,50 0/1 * * Jan-Dec * 2020-2100".to_string()),
         }
     }
 }
 ```
 
There's a lot more in the [examples] directory.


## License

Licensed under either of

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)


## To Do List
- [x] Support tokio Ecology.
- [x] Disable unwrap related methods that will panic.
- [ ] Thread and running task quit when delayTimer drop.
- [ ] neaten todo in code, replenish tests and benchmark.
- [ ] batch-opration.
- [x] report-for-server.
- [ ] TASK-TAG.
- [ ] Future upgrade of delay_timer to multi-wheel mode, different excutor handling different wheels e.g. subtract laps for one wheel, run task for one wheel.

#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.


#### The author comments:

#### Make an upgrade plan for smooth updates in the future, Such as stop serve  back-up ` unfinished task`  then up new version serve load task.bak, Runing.
