
//! A "prelude" for users of the `delay-timer` crate.
//!
//! This prelude is similar to the standard library's prelude in that you'll
//! almost always want to import its entire contents, but unlike the standard
//! library's prelude you'll have to do so manually:
//!
//! ```
//! # #![allow(warnings)]
//! use delay_timer::prelude::*;
//! ```
//!
//! The prelude may grow over time as additional items see ubiquitous use.

pub use anyhow::Result as AnyResult;
pub use cron_clock;
pub use crate::macros::*;
pub use crate::timer::runtime_trace::task_handle::DelayTaskHandler;
pub use crate::timer::task::{Frequency, Task, TaskBuilder};
pub use crate::utils::convenience::cron_expression_grammatical_candy::CronCandy;
pub use crate::delay_timer::{DelayTimer, get_timestamp};
pub use crate::utils::convenience::functions::create_default_delay_task_handler;

cfg_tokio_support!(
    pub use tokio::task::spawn as async_spawn;
    pub use tokio::task::spawn_blocking as unblock_spawn;
    pub(crate) use tokio::sync::mpsc::{
        UnboundedReceiver as AsyncReceiver, UnboundedSender as AsyncSender,
    };
    pub(crate) use tokio::sync::Mutex as AsyncMutex;
    pub(crate) use tokio::task::yield_now;
);

cfg_smol_support!(
    pub(crate) use smol::channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
    pub(crate) use smol::lock::Mutex as AsyncMutex;
    pub(crate) use smol::future::yield_now;
    pub(crate) use smol::{channel::unbounded as async_unbounded_channel, future::block_on};
    pub use smol::future as future_lite;
    pub use smol::spawn as async_spawn;
    pub use smol::unblock as unblock_spawn;
);
