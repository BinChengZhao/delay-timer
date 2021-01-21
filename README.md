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

 #[macro_use]
 use delay_timer::prelude::*;

 use std::str::FromStr;
 use std::sync::atomic::{
     AtomicUsize,
     Ordering::{Acquire, Release},
 };
 use std::sync::{atomic::AtomicI32, Arc};
 use std::thread::{self, park_timeout};
 use std::time::Duration;
 use smol::Timer;
 use hyper::{Client, Uri};

 fn main() {
     let delay_timer = DelayTimerBuilder::default().build();

     // Add an asynchronous task to delay_timer.
     delay_timer.add_task(build_task(TaskBuilder::default()));
     // Since the tasks are executed in 8-second cycles,
     // we deal with something else.
     // Do someting about 8s.
     thread::sleep(Duration::new(8, 1_000_000));
     delay_timer.remove_task(1);
     delay_timer.stop_delay_timer();
 }

 fn build_task(mut task_builder: TaskBuilder) -> Task {
     let body = create_async_fn_body!({
         let mut res = surf::get("https://httpbin.org/get").await.unwrap();
         dbg!(res.body_string().await.unwrap());
     });

     task_builder
         .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::PerEightSeconds))
         .set_task_id(2)
         .set_maximum_running_time(5)
         .spawn(body)
         .unwrap()
 }
 
 enum AuspiciousTime {
     PerSevenSeconds,
     PerEightSeconds,
     LoveTime,
 }

 impl Into<CandyCronStr> for AuspiciousTime {
     fn into(self) -> CandyCronStr {
         match self {
             Self::PerSevenSeconds => CandyCronStr("0/7 * * * * * *"),
             Self::PerEightSeconds => CandyCronStr("0/8 * * * * * *"),
             Self::LoveTime => CandyCronStr("0,10,15,25,50 0/1 * * Jan-Dec * 2020-2100"),
         }
     }
 }
 ```


 Capture the specified environment information and build the closure & task:

 ``` rust
 #[macro_use]
 use delay_timer::prelude::*;

 use std::str::FromStr;
 use std::sync::atomic::{
     AtomicUsize,
     Ordering::{Acquire, Release},
 };
 use std::sync::{atomic::AtomicI32, Arc};
 use std::thread::{self, park_timeout};
 use std::time::Duration;
 use smol::Timer;
 use hyper::{Client, Uri};


 let delay_timer = DelayTimer::new();

 let share_num = Arc::new(AtomicUsize::new(0));
 let share_num_bunshin = share_num.clone();

 let body = create_async_fn_body!((share_num_bunshin){
     share_num_bunshin_ref.fetch_add(1, Release);
     Timer::after(Duration::from_secs(9)).await;
     share_num_bunshin_ref.fetch_sub(1, Release);
 });

 let task = TaskBuilder::default()
     .set_frequency_by_candy(CandyFrequency::CountDown(9, CandyCron::Secondly))
     .set_task_id(1)
     .set_maximun_parallel_runable_num(3)
     .spawn(body)
     .unwrap();

 delay_timer.add_task(task).unwrap();

 ```



 Building dynamic future tasks:
 ``` rust
 #[macro_use]
 use delay_timer::prelude::*;

 use std::str::FromStr;
 use std::sync::atomic::{
     AtomicUsize,
     Ordering::{Acquire, Release},
 };
 use std::sync::{atomic::AtomicI32, Arc};
 use std::thread::{self, park_timeout};
 use std::time::Duration;
 use smol::Timer;
 use hyper::{Client, Uri};

 fn build_task(mut task_builder: TaskBuilder) -> Task {
     let body = generate_closure_template(String::from("dynamic"));

     task_builder
         .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::PerEightSeconds))
         .set_task_id(2)
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
             future_inner.await;
             context.finishe_task().await;
         };

         create_delay_task_handler(async_spawn_by_tokio(future))
     }
 }

 pub async fn async_template(id: i32, name: String) {
     let client = Client::new();

     let url = format!("http://httpbin.org/get?id={}&name={}", id, name);
      let uri: Uri = url.parse().unwrap();
      let res = client.get(uri).await.unwrap();
      println!("Response: {}", res.status());
      // Concatenate the body stream into a single buffer...
      let buf = hyper::body::to_bytes(res).await.unwrap();
      println!("body: {:?}", buf);
  }
  enum AuspiciousTime {
      PerSevenSeconds,
      PerEightSeconds,
      LoveTime,
  }
 
  impl Into<CandyCronStr> for AuspiciousTime {
      fn into(self) -> CandyCronStr {
          match self {
              Self::PerSevenSeconds => CandyCronStr("0/7 * * * * * *"),
              Self::PerEightSeconds => CandyCronStr("0/8 * * * * * *"),
              Self::LoveTime => CandyCronStr("0,10,15,25,50 0/1 * * Jan-Dec * 2020-2100"),
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
- [ ] Disable unwrap related methods that will panic.
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
