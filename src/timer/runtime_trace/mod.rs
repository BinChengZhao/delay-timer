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
//! 2. If the task is not set `max-running-time`, it will be automatically recycled when it finishes running.
//! 3. The internal-task-handle, which holds the execution handle of the running task,
//! gives lib the support to exit the task at any time.
pub(crate) mod sweeper;
pub mod task_handle;
pub mod task_instance;
