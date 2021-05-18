//! Public error of delay-timer..

use crate::prelude::*;

/// Error enumeration for `Task`-related operations.
#[derive(Error, Debug)]
// TODO: Added implementation of Trait for PartialEq, Eq, Clone, etc.
pub enum TaskError {
    /// Error variant for Cron expression parsing.
    #[error("Cron expression analysis error.")]
    CronExpressionAnalyzeError(#[from] CronExpressionAnalyzeError),
}

/// Error enumeration for Cron expression parsing.
#[derive(Error, Debug)]
pub enum CronExpressionAnalyzeError {
    /// Access to thread local storage failed.
    #[error("Thread local storage access failed.")]
    DisAccess(#[from] std::thread::AccessError),
    /// Irregular cron expressions that cause parsing failures.
    #[error("The cron expression was parsed incorrectly.")]
    DisParse(#[from] cron_error::Error),
}

/// Error enumeration for Handle Instance.
#[derive(Error, PartialEq, Eq, Clone, Copy, Debug)]
pub enum HandleInstanceError {
    /// Missing event sender `timer_event_sender`.
    #[error("Missing `timer_event_sender`.")]
    MisEventSender,
    #[error("Task instance channel exception.")]
    /// Internal channel communication abnormality.
    InternalChannelAnomalyT(#[from] channel::TryRecvError),
    /// Internal channel communication abnormality.
    #[error("Task instance channel exception.")]
    InternalChannelAnomaly(#[from] channel::RecvError),
    /// Running instance of the task is no longer maintained.
    #[error("Running instance of the task is no longer maintained.")]
    Expired,
}
