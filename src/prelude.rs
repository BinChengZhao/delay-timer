//! A "prelude" for users of the `delay-timer` crate.
//!
//! This prelude is similar to the standard library's prelude in that you'll
//! almost always want to import its entire contents, but unlike the standard
//! library's prelude you'll have to do so manually:
//!
//! ```
//! use delay_timer::prelude::*;
//! ```
//!
//! The prelude may grow over time as additional items see ubiquitous use.

pub use crate::entity::{get_timestamp, get_timestamp_micros, DelayTimer, DelayTimerBuilder};
pub use crate::macros::*;
pub use crate::timer::runtime_trace::task_handle::DelayTaskHandler;
pub use crate::timer::runtime_trace::task_instance::{Instance, TaskInstancesChain};
pub use crate::timer::task::TaskContext;
pub use crate::timer::task::{Frequency, ScheduleIteratorTimeZone, Task, TaskBuilder};
pub use crate::timer::timer_core::TimerEvent;
pub use crate::utils::convenience::cron_expression_grammatical_candy::{
    CandyCron, CandyCronStr, CandyFrequency,
};
pub use crate::utils::convenience::functions::{
    create_default_delay_task_handler, create_delay_task_handler, unblock_process_task_fn,
};

pub use anyhow::Result as AnyResult;
pub use cron_clock::{self, FixedOffset, Local, TimeZone, Utc};
pub use smol::future as future_lite;
pub use smol::spawn as async_spawn;
pub use smol::unblock as unblock_spawn;

pub(crate) use crate::entity::RuntimeKind;
pub(crate) use crate::timer::event_handle::SharedHeader;
pub(crate) use crate::timer::runtime_trace::task_handle::DelayTaskHandlerBox;
pub(crate) use crate::timer::runtime_trace::task_handle::DelayTaskHandlerBoxBuilder;
pub(crate) use crate::timer::runtime_trace::task_instance::{
    InstanceList, InstanceListInner, TaskInstancesChainMaintainer,
};

pub(crate) use crate::utils::parse::shell_command::{ChildGuard, ChildGuardList};
/// Global event sender that provides events to `EventHandle` processing.
/// Use Option-None to overcome  `only be initialized with constant functions and values`.
pub(crate) static mut GLOBAL_TIMER_EVENT_SENDER: Option<TimerEventSender> = None;

pub(crate) use smol::channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
pub(crate) use smol::future::yield_now;
pub(crate) use smol::lock::{Mutex as AsyncMutex, RwLock as AsyncRwLock};

pub(crate) type TimerEventSender = AsyncSender<TimerEvent>;
pub(crate) type TimerEventReceiver = AsyncReceiver<TimerEvent>;

cfg_tokio_support!(
    pub use tokio::task::spawn as async_spawn_by_tokio;
    pub use tokio::task::spawn_blocking as unblock_spawn_by_tokio;
    pub use tokio::time::sleep as sleep_by_tokio;
    pub use crate::utils::convenience::functions::tokio_unblock_process_task_fn;
);

cfg_status_report!(
    pub use crate::utils::status_report::PublicEvent;
);
