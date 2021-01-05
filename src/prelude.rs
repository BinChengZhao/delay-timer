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

pub use crate::entity::{get_timestamp, DelayTimer, DelayTimerBuilder};
pub use crate::macros::*;
pub use crate::timer::runtime_trace::task_handle::DelayTaskHandler;
pub use crate::timer::task::TaskContext;
pub use crate::timer::task::{Frequency, Task, TaskBuilder};
pub use crate::timer::timer_core::TimerEvent;
pub use crate::utils::convenience::cron_expression_grammatical_candy::{
    CandyCron, CandyCronStr, CandyFrequency,
};
pub use crate::utils::convenience::functions::{
    create_default_delay_task_handler, create_delay_task_handler, unblock_process_task_fn,
};
pub use anyhow::Result as AnyResult;
pub use cron_clock;
pub use smol::future as future_lite;
pub use smol::spawn as async_spawn;
pub use smol::unblock as unblock_spawn;

//TODO:Try to unify with smol channel.
pub(crate) use crate::entity::RuntimeKind;
pub(crate) use smol::channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
pub(crate) use smol::future::yield_now;
pub(crate) use smol::lock::Mutex as AsyncMutex;

pub(crate) type TimerEventSender = AsyncSender<TimerEvent>;
pub(crate) type TimerEventReceiver = AsyncReceiver<TimerEvent>;

cfg_tokio_support!(
    pub use tokio::task::spawn as async_spawn_by_tokio;
    pub use tokio::task::spawn_blocking as unblock_spawn_by_tokio;
    pub use tokio::time::sleep as sleep_by_tokio;
);

cfg_status_report!(
    pub use crate::utils::status_report::PublicEvent;
);
