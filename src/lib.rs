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
