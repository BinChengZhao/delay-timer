//! timer is the core module of the library , it can provide an API for task building ,
//! task scheduling , event handling , resource recovery .

pub mod task;
pub mod timer_core;

pub(crate) mod event_handle;
pub(crate) mod runtime_trace;
pub(crate) mod slot;

pub(crate) use slot::Slot;
pub(crate) use task::{Task, TaskMark};
