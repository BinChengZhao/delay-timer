//! Public error of delay-timer..

use crate::prelude::*;

/// Error enumeration for `Task`-related operations.
#[derive(Error, Debug)]
pub enum TaskError {
    /// Error variant for Cron expression parsing.
    #[error("Cron expression analysis error.")]
    CronExpressionAnalyzeError(#[from] CronExpressionAnalyzeError),
    /// Task sending failure.
    #[error("Task sending failure.")]
    DisSend(#[from] channel::TrySendError<TimerEvent>),
    /// Task event get failed.
    #[error("Task event get failed.")]
    DisGetEvent(#[from] channel::TryRecvError),
}

/// Error enumeration for `TaskInstance`-related operations.
#[derive(Error, Debug)]
pub enum TaskInstanceError {
    /// TaskInstance sending failure.
    #[error("TaskInstance sending failure.")]
    DisSend(#[from] channel::TrySendError<TimerEvent>),
    /// TaskInstance event get failed.
    #[error("TaskInstance event get failed.")]
    DisGetEvent(#[from] channel::TryRecvError),
    /// TaskInstance cancel failure.
    #[error("The task has been (completed or canceled) and cannot be cancelled.")]
    DisCancel,
    /// TaskInstance cancel TimeOut.
    #[error("Waiting for cancellation timeout.")]
    DisCancelTimeOut,
    /// Missing event sender `timer_event_sender`.
    #[error("Missing `timer_event_sender`.")]
    MisEventSender,
    /// Internal channel communication abnormality.
    #[error("Task instance channel exception.")]
    InternalChannelAnomaly(#[from] channel::RecvError),
    /// Running instance of the task is no longer maintained.
    #[error("Running instance of the task is no longer maintained.")]
    Expired,
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
