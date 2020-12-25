//! DelayTimer is a cyclic task manager with latency properties,
//! based on an internal event manager and task scheduler,
//! and supported by the runtime provided by smol,
//! which makes it easy to manage asynchronous/synchronous/scripted cyclic tasks.
//!
//! # DelayTimer
//!
//! User applications can be served through the lib used by DelayTimer:
//!
//! 1. Mission deployment.
//!
//! Except for the simple execution in a few seconds

//!，还可以指定 特定日期，比如周日的凌晨4点执行一个备份任务

//!还支持判断前一个任务没执行完成时，是否并行，最大并行数量

//!同步异步任务都支持， 支持执行过程中取消

//!支持 smol tokio async-std构建的Future等
//!
#![feature(split_inclusive)]
#![feature(linked_list_cursors)]
#![feature(associated_type_defaults)]
//TODO:When the version is stable in the future, we should consider using stable compile unified.

pub(crate) use utils::parse::shell_command::{ChildGuard, ChildGuardList};

#[macro_use]
pub mod macros;
pub mod entity;
pub mod prelude;
pub mod timer;
pub mod utils;

//TODO: Maybe can independent bench mod to one project.
//Or via `rustversion` Isolation of different modules.
//TODO: Add a prelude.
//FIXME:发版之前：dev-依赖：hyper，更正为支持 tokio ~0.3.* 的版本.
