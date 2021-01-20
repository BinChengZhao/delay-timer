//! DelayTimer like crontab is a cyclic task manager with latency properties,
//! but synchronous asynchronous tasks are possible,
//! based on an internal event manager and task scheduler,
//! and supported by the runtime provided by smol and tokio,
//! which makes it easy to manage and dynamic add/cancel/remove is supported.
//!
//! - [TaskBuilder](crate::timer::task::TaskBuilder) It is a builder for Task
//!   that provides APIs for setting such as maximum number of parallel runs,
//!   run content, run identity, run duration, etc.
//! - [DelayTimerBuilder](crate::entity::DelayTimerBuilder) It is a builder for
//!   [DelayTimer](crate::entity::DelayTimer) that provides APIs for setting such as customization runtime
//!   and enable status-report.
//!
//!
//!
//! # Usage
//!
//! First, add this to your Cargo.toml
//!
//! ```toml
//! [dependencies]
//! delay_timer = "0.2.0"
//! ```
//!
//! Next:
//!
//! ``` rust
//!
//! #[macro_use]
//! use delay_timer::prelude::*;
//!
//! use std::str::FromStr;
//! use std::sync::atomic::{
//!     AtomicUsize,
//!     Ordering::{Acquire, Release},
//! };
//! use std::sync::{atomic::AtomicI32, Arc};
//! use std::thread::{self, park_timeout};
//! use std::time::Duration;
//! use smol::Timer;
//! use hyper::{Client, Uri};
//!
//! fn main() {
//!     let delay_timer = DelayTimerBuilder::default().build();
//!
//!     // Add an asynchronous task to delay_timer.
//!     delay_timer.add_task(build_task(TaskBuilder::default()));
//!     // Since the tasks are executed in 8-second cycles,
//!     // we deal with something else.
//!     // Do someting about 8s.
//!     thread::sleep(Duration::new(8, 1_000_000));
//!     delay_timer.remove_task(1);
//!     delay_timer.stop_delay_timer();
//! }
//!
//! fn build_task(mut task_builder: TaskBuilder) -> Task {
//!     let body = create_async_fn_body!({
//!         let mut res = surf::get("https://httpbin.org/get").await.unwrap();
//!         dbg!(res.body_string().await.unwrap());
//!     });
//!
//!     task_builder
//!         .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::PerEightSeconds))
//!         .set_task_id(2)
//!         .set_maximum_running_time(5)
//!         .spawn(body)
//!         .unwrap()
//! }
//! enum AuspiciousTime {
//!     PerSevenSeconds,
//!     PerEightSeconds,
//!     LoveTime,
//! }
//!
//! impl Into<CandyCronStr> for AuspiciousTime {
//!     fn into(self) -> CandyCronStr {
//!         match self {
//!             Self::PerSevenSeconds => CandyCronStr("0/7 * * * * * *"),
//!             Self::PerEightSeconds => CandyCronStr("0/8 * * * * * *"),
//!             Self::LoveTime => CandyCronStr("0,10,15,25,50 0/1 * * Jan-Dec * 2020-2100"),
//!         }
//!     }
//! }
//! ```
//!
//!
//!
//! Capture the specified environment information and build the closure & task:
//! ``` rust
//! #[macro_use]
//! use delay_timer::prelude::*;
//!
//! use std::str::FromStr;
//! use std::sync::atomic::{
//!     AtomicUsize,
//!     Ordering::{Acquire, Release},
//! };
//! use std::sync::{atomic::AtomicI32, Arc};
//! use std::thread::{self, park_timeout};
//! use std::time::Duration;
//! use smol::Timer;
//! use hyper::{Client, Uri};
//!
//!
//! let delay_timer = DelayTimer::new();
//!
//! let share_num = Arc::new(AtomicUsize::new(0));
//! let share_num_bunshin = share_num.clone();
//!
//! let body = create_async_fn_body!((share_num_bunshin){
//!     share_num_bunshin_ref.fetch_add(1, Release);
//!     Timer::after(Duration::from_secs(9)).await;
//!     share_num_bunshin_ref.fetch_sub(1, Release);
//! });
//!
//! let task = TaskBuilder::default()
//!     .set_frequency_by_candy(CandyFrequency::CountDown(9, CandyCron::Secondly))
//!     .set_task_id(1)
//!     .set_maximun_parallel_runable_num(3)
//!     .spawn(body)
//!     .unwrap();
//!
//! delay_timer.add_task(task).unwrap();
//!
//! ```
//!
//!
//!
//! Building dynamic future tasks:
//! ```
//! #[macro_use]
//! use delay_timer::prelude::*;
//!
//! use std::str::FromStr;
//! use std::sync::atomic::{
//!     AtomicUsize,
//!     Ordering::{Acquire, Release},
//! };
//! use std::sync::{atomic::AtomicI32, Arc};
//! use std::thread::{self, park_timeout};
//! use std::time::Duration;
//! use smol::Timer;
//! use hyper::{Client, Uri};
//!
//!
//!
//! fn build_task(mut task_builder: TaskBuilder) -> Task {
//!     let body = generate_closure_template(String::from("dynamic"));
//!
//!     task_builder
//!         .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::PerEightSeconds))
//!         .set_task_id(2)
//!         .set_maximum_running_time(5)
//!         .spawn(body)
//!         .unwrap()
//! }
//!
//! pub fn generate_closure_template(
//!     name: String,
//! ) -> impl Fn(TaskContext) -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
//!     move |context| {
//!         let future_inner = async_template(get_timestamp() as i32, name.clone());
//!
//!         let future = async move {
//!             future_inner.await;
//!             context.finishe_task().await;
//!         };
//!
//!         create_delay_task_handler(async_spawn_by_tokio(future))
//!     }
//! }
//!
//! pub async fn async_template(id: i32, name: String) {
//!     let client = Client::new();
//!
//!     let url = format!("http://httpbin.org/get?id={}&name={}", id, name);
//!     let uri: Uri = url.parse().unwrap();
//!     let res = client.get(uri).await.unwrap();
//!     println!("Response: {}", res.status());
//!     // Concatenate the body stream into a single buffer...
//!     let buf = hyper::body::to_bytes(res).await.unwrap();
//!     println!("body: {:?}", buf);
//! }
//! enum AuspiciousTime {
//!     PerSevenSeconds,
//!     PerEightSeconds,
//!     LoveTime,
//! }
//!
//! impl Into<CandyCronStr> for AuspiciousTime {
//!     fn into(self) -> CandyCronStr {
//!         match self {
//!             Self::PerSevenSeconds => CandyCronStr("0/7 * * * * * *"),
//!             Self::PerEightSeconds => CandyCronStr("0/8 * * * * * *"),
//!             Self::LoveTime => CandyCronStr("0,10,15,25,50 0/1 * * Jan-Dec * 2020-2100"),
//!         }
//!     }
//! }
//! ```

#![feature(linked_list_cursors)]
// Backup : https://github.com/contain-rs/linked-list/blob/master/src/lib.rs

// TODO:When the version is stable in the future, we should consider using stable compile unified.
// FIXME: Auto fill cli-args `features = full` when exec cargo test.
#[macro_use]
pub mod macros;
pub mod entity;
pub mod prelude;
pub mod timer;
pub mod utils;

pub use cron_clock;
