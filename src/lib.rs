#![feature(split_inclusive)]
#![feature(str_strip)]
#![feature(drain_filter)]
#![feature(no_more_cas)]
// #[allow(dead_code)]
pub mod delay_timer;
pub mod generate_fn_macro;
pub mod timer;
pub mod utils;

#[macro_use]
extern crate lazy_static;

pub use generate_fn_macro::*;
//待办项-补充到 readme To Do List
//TODO: Use AsyncMutex.  task.rs - TASKMAP
