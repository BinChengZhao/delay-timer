//! kind of contanier for trace task-handle.
//!
//! # TaskTrace
//!
//! storage many task handle you can cancel running task by task-id and record-id:
//!
//! 1. task-id and record-id should be unique.
//!
//! # Sweeper
//!
//! this is the recource guardian:
//!
//! 1. recycle recource get through small-heap by task setting max-running-time.
pub(crate) mod sweeper;
pub mod task_handle;
