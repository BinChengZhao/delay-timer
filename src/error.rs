//! Public error of delay-timer..

use crate::prelude::*;

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
