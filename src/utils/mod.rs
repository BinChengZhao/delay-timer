//! utils is a tool module that provides easy shell-command parsing,
//! and functions that generate closures.
pub mod convenience;
pub mod parse;

cfg_status_report!(
    pub mod status_report;
);

pub use convenience::functions;
pub use parse::shell_command::{parse_and_run, parse_and_runx};
