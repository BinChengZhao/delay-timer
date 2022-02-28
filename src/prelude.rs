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

pub use crate::entity::{timestamp, timestamp_micros, DelayTimer, DelayTimerBuilder};
pub use crate::error::*;
pub use crate::timer::runtime_trace::state::instance;
pub use crate::timer::runtime_trace::task_handle::DelayTaskHandler;
pub use crate::timer::runtime_trace::task_instance::{Instance, TaskInstance, TaskInstancesChain};
pub use crate::timer::task::TaskContext;
pub use crate::timer::task::{
    FrequencyCronStr as Frequency, ScheduleIteratorTimeZone, Task, TaskBuilder,
};
pub use crate::timer::timer_core::{FinishOutput, FinishTaskBody, TimerEvent};

pub use crate::utils::convenience::cron_expression_grammatical_candy::{
    CandyCron, CandyCronStr, CandyFrequency,
};
pub use crate::utils::convenience::functions::{
    create_default_delay_task_handler, create_delay_task_handler,
};

pub use anyhow::{anyhow, Result as AnyResult};
pub use cron_clock::{self, error as cron_error, FixedOffset, Local, TimeZone, Utc};
pub use smol::channel;
pub use smol::future as future_lite;
pub use smol::spawn as async_spawn_by_smol;
pub use smol::unblock as unblock_spawn_by_smol;
pub use smol::Task as SmolJoinHandler;
pub use thiserror::Error;

/// State of the task run instance.
pub type InstanceState = usize;

pub(crate) use crate::entity::RuntimeKind;
pub(crate) use crate::timer::event_handle::SharedHeader;
pub(crate) use crate::timer::runtime_trace::state;
pub(crate) use crate::timer::runtime_trace::task_handle::DelayTaskHandlerBox;
pub(crate) use crate::timer::runtime_trace::task_handle::DelayTaskHandlerBoxBuilder;
pub(crate) use crate::timer::runtime_trace::task_instance::TaskInstancesChainMaintainer;
pub(crate) use crate::timer::task::Routine;

pub(crate) use crate::utils::parse::shell_command::{ChildGuard, ChildGuardList, ChildUnify};
pub(crate) use dashmap::DashMap;
pub(crate) use log::{debug, error, info, trace};
pub(crate) use smol::channel::{unbounded, Receiver as AsyncReceiver, Sender as AsyncSender};
pub(crate) use smol::future::yield_now;
pub(crate) use smol::lock::Mutex as AsyncMutex;
pub(crate) use smol::Timer as AsyncTimer;
pub(crate) use std::convert::{TryFrom, TryInto};
pub(crate) use std::future::Future;
pub(crate) use std::iter::StepBy;
pub(crate) use std::ops::RangeFrom;
pub(crate) use std::process::ExitStatus;
pub(crate) use std::time::Duration;
pub(crate) use tracing::{info_span, instrument, Instrument};

pub(crate) type SecondsState = StepBy<RangeFrom<u64>>;
pub(crate) type TimerEventSender = AsyncSender<TimerEvent>;
pub(crate) type TimerEventReceiver = AsyncReceiver<TimerEvent>;

pub use tokio::task::{
    spawn as async_spawn_by_tokio, spawn_blocking as unblock_spawn_by_tokio,
    JoinHandle as TokioJoinHandle,
};
pub use tokio::time::sleep as sleep_by_tokio;

cfg_status_report!(
    pub use crate::utils::status_report::PublicEvent;
    pub(crate) use crate::utils::status_report::GLOBAL_STATUS_REPORTER;
);

#[cfg(target_family = "unix")]
pub(crate) use std::os::unix::process::ExitStatusExt;
#[cfg(target_family = "windows")]
pub(crate) use std::os::windows::process::ExitStatusExt;

pub(crate) const ONE_SECOND: u64 = 1;
pub(crate) const ONE_MINUTE: u64 = ONE_SECOND * 60;
pub(crate) const ONE_HOUR: u64 = ONE_MINUTE * 60;
pub(crate) const ONE_DAY: u64 = ONE_HOUR * 24;
