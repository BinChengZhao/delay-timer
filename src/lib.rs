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
//!   DelayTimer that provides APIs for setting such as customization runtime
//!   and enable status-report.
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
//! fn main() {
//!     let delay_timer = DelayTimerBuilder::default().build();
//!
//!     // Add an asynchronous task to delay_timer.
//!     delay_timer.add_task(build_task(TaskBuilder::default()));
//!     // Since the tasks are executed in 8-second cycles,
//!     // we deal with something else.
//!     sleep(Duration::new(8, 1_000_000));
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
//! ```
#![feature(split_inclusive)]
#![feature(linked_list_cursors)]

//TODO:When the version is stable in the future, we should consider using stable compile unified.

#[macro_use]
pub mod macros;
pub mod entity;
pub mod prelude;
pub mod timer;
pub mod utils;
