// all the original `DelayTimerBuilder`s have to add a call `tokio_runtime_by_smol`.

//! DelayTimer like crontab is a cyclic task manager with latency properties,
//! but synchronous asynchronous tasks are possible,
//! based on an internal event manager and task scheduler,
//! and supported by the runtime provided by smol and tokio,
//! which makes it easy to manage and dynamic add/cancel/remove is supported.
//!
//! - [TaskBuilder](crate::timer::task::TaskBuilder) It is a builder for [Task](crate::timer::task::Task)
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
//! delay_timer = "*"
//! ```
//!
//! Next:
//!
//! ``` rust
//!
//! use anyhow::Result;
//! use delay_timer::prelude::*;
//! use std::time::Duration;
//! use smol::Timer;
//!
//! fn main() -> Result<()> {
//!     // Build an DelayTimer that uses the default configuration of the `smol` runtime internally.
//!     let delay_timer = DelayTimerBuilder::default().build();
//!
//!     // Develop a print job that runs in an asynchronous cycle.
//!     // A chain of task instances.
//!     let task_instance_chain = delay_timer.insert_task(build_task_async_print()?)?;
//!
//!     // Get the running instance of task 1.
//!     let task_instance = task_instance_chain.next_with_wait()?;
//!
//!     // Cancel running task instances.
//!     task_instance.cancel_with_wait()?;
//!
//!     // Remove task which id is 1.
//!     delay_timer.remove_task(1)?;
//!
//!     // No new tasks are accepted; running tasks are not affected.
//!     delay_timer.stop_delay_timer()?;
//!
//!     Ok(())
//! }
//!
//! fn build_task_async_print() -> Result<Task, TaskError> {
//!    let mut task_builder = TaskBuilder::default();
//!
//!    let body = || async {
//!        println!("create_async_fn_body!");
//!
//!        Timer::after(Duration::from_secs(3)).await;
//!
//!        println!("create_async_fn_body:i'success");
//!    };
//!
//!    task_builder
//!        .set_task_id(1)
//!        .set_frequency_repeated_by_seconds(1)
//!        .set_maximum_parallel_runnable_num(2)
//!        .spawn_async_routine(body)
//! }
//!
//! ```
//!
//!
//! Use in asynchronous contexts.
//! ```
//!
//! use delay_timer::prelude::*;
//!
//! use anyhow::Result;
//!
//! use smol::Timer;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // In addition to the mixed (smol & tokio) runtime
//!     // You can also share a tokio runtime with delayTimer, please see api `DelayTimerBuilder::tokio_runtime` for details.
//!
//!     // Build an DelayTimer that uses the default configuration of the Smol runtime internally.
//!     let delay_timer = DelayTimerBuilder::default().build();
//!
//!     // Develop a print job that runs in an asynchronous cycle.
//!     let task_instance_chain = delay_timer.insert_task(build_task_async_print()?)?;
//!
//!     // Get the running instance of task 1.
//!     let task_instance = task_instance_chain.next_with_async_wait().await?;
//!
//!     // Cancel running task instances.
//!     task_instance.cancel_with_async_wait().await?;
//!
//!
//!     // Remove task which id is 1.
//!     delay_timer.remove_task(1)?;
//!
//!     // No new tasks are accepted; running tasks are not affected.
//!     Ok(delay_timer.stop_delay_timer()?)
//! }
//!
//! fn build_task_async_print() -> Result<Task, TaskError> {
//!     let mut task_builder = TaskBuilder::default();
//!
//!     let body = || async {
//!         println!("create_async_fn_body!");
//!
//!         Timer::after(Duration::from_secs(3)).await;
//!
//!         println!("create_async_fn_body:i'success");
//!     };
//!
//!     task_builder
//!         .set_task_id(1)
//!         .set_frequency_repeated_by_seconds(6)
//!         .set_maximum_parallel_runnable_num(2)
//!         .spawn_async_routine(body)
//! }
//!
//!
//!
//!
//! ```
//! Capture the specified environment information and build the closure & task:
//! ```
//! #[macro_use]
//! use delay_timer::prelude::*;
//!
//! use std::sync::atomic::{
//!     AtomicUsize,
//!     Ordering::{Acquire, Release},
//! };
//! use std::sync::Arc;
//! use std::time::Duration;
//! use smol::Timer;
//!
//!
//! let delay_timer = DelayTimer::new();
//!
//! let share_num = Arc::new(AtomicUsize::new(0));
//! let share_num_bunshin = share_num.clone();
//!
//! let body = move || {
//!     let share_num_bunshin_ref = share_num_bunshin.clone();
//!     async move {
//!         share_num_bunshin_ref.fetch_add(1, Release);
//!         Timer::after(Duration::from_secs(9)).await;
//!         share_num_bunshin_ref.fetch_sub(1, Release);
//!     }
//! };
//!
//!
//! let task = TaskBuilder::default()
//!     .set_frequency_count_down_by_seconds(1, 9)
//!     .set_task_id(1)
//!     .set_maximum_parallel_runnable_num(3)
//!     .spawn_async_routine(body).expect("");
//!
//! delay_timer.add_task(task).ok();
//!
//! ```
//!
//!
//!
//! Building dynamic future tasks:
//  TODO: We cannot set the `required-features` of toml-tag on a document test.
//  Wating for `https://doc.rust-lang.org/rustdoc/documentation-tests.html` update.
//! ```
//! #[macro_use]
//! use delay_timer::prelude::*;
//! use anyhow::Result;
//! use std::str::FromStr;
//! use std::sync::atomic::{
//!     AtomicUsize,
//!     Ordering::{Acquire, Release},
//! };
//! use std::sync::{atomic::AtomicI32, Arc};
//! use std::thread::{self, park_timeout};
//! use std::time::Duration;
//! use smol::Timer;
//! use tokio::time::sleep;
//! use hyper::{Client, Uri};
//!
//!
//!
//! fn build_task(mut task_builder: TaskBuilder, name:String, id:i32) -> Result<Task, TaskError> {
//!     let body = move || {
//!         let name_ref = name.clone();
//!         async move {
//!             async_template(id, name_ref).await.expect("Request failed.");
//!
//!             sleep(Duration::from_secs(3)).await;
//!
//!             println!("create_async_fn_body:i'success");
//!         }
//!     };
//!
//!     task_builder
//!         .set_frequency_repeated_by_seconds(8)
//!         .set_task_id(2)
//!         .set_maximum_running_time(5)
//!         .spawn_async_routine(body)
//! }
//!
//!
//!  pub async fn async_template(id: i32, name: String) -> Result<()> {
//!      let client = Client::new();
//!  
//!      // The default connector does not handle TLS.
//!      // Speaking to https destinations will require configuring a connector that implements TLS.
//!      // So use http for test.
//!      let url = format!("http://httpbin.org/get?id={}&name={}", id, name);
//!      let uri: Uri = url.parse()?;
//!  
//!      let res = client.get(uri).await?;
//!      println!("Response: {}", res.status());
//!      // Concatenate the body stream into a single buffer...
//!      let buf = hyper::body::to_bytes(res).await?;
//!      println!("body: {:?}", buf);
//!      Ok(())
//!  }
//!
//! ```
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![cfg_attr(RUSTC_IS_NIGHTLY, feature(linked_list_cursors))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#[macro_use]
pub mod macros;
pub mod entity;
pub mod error;
pub mod prelude;
pub mod timer;
pub mod utils;

pub use anyhow;
pub use cron_clock;
pub use snowflake;
