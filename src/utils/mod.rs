//! utils is a tool module that provides easy shell-command parsing,
//! and functions that generate closures.
pub mod convenience;
pub mod parse;
#[cfg(feature = "status-report")]
pub mod status_report;

#[cfg(feature = "status-report")]
pub use status_report::statusReport;

pub use convenience::functions;
pub use parse::shell_command::parse_and_run;
