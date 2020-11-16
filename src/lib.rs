//! DelayTimer is a cyclic task manager with latency properties,
//! based on an internal event manager and task scheduler,
//! and supported by the runtime provided by smol,
//! which makes it easy to manage asynchronous/synchronous/scripted cyclic tasks.
//!
//! # DelayTimer
//!
//! User applications can be served through the lib used by DelayTimer:
//!
//! 1. Mission deployment.
#![feature(split_inclusive)]
#![feature(drain_filter)]
#![feature(test)]
//TODO:When the version is stable in the future, we should consider using stable compile unified.
extern crate test;

pub(crate) use utils::parse::shell_command::{ChildGuard, ChildGuardList};

pub mod delay_timer;
pub mod generate_fn_macro;
pub mod timer;
pub mod utils;

pub use anyhow::Result as AnyResult;
pub use cron_clock;
pub use generate_fn_macro::*;
pub use smol::future as future_lite;
pub use smol::spawn as async_spawn;
pub use smol::unblock as unblock_spawn;
pub use timer::runtime_trace::task_handle::DelayTaskHandler;
pub use timer::task::{Frequency, Task, TaskBuilder};
//TODO: Maybe can independent bench mod to one project.
#[cfg(test)]
mod tests {
    use smol::{channel::unbounded, future::block_on};

    use crate::{
        delay_timer::SharedHeader,
        timer::{
            task::{Frequency, TaskBuilder},
            timer_core::{Timer, TimerEvent},
        },
        utils::functions::create_default_delay_task_handler,
    };

    use test::Bencher;

    #[bench]
    fn bench_task_spwan(b: &mut Bencher) {
        let body = move || create_default_delay_task_handler();

        let mut task_builder = TaskBuilder::default();
        task_builder
            .set_frequency(Frequency::CountDown(1, "@yearly"))
            .set_maximum_running_time(5)
            .set_task_id(1);

        // String parsing to corn-expression -> iterator is the most time-consuming operation.
        // The iterator is used to find out when the next execution should take place, in about 500 ns.
        b.iter(|| task_builder.spawn(body.clone()));
    }

    #[bench]
    fn bench_maintain_task(b: &mut Bencher) {
        let (timer_event_sender, _timer_event_receiver) = unbounded::<TimerEvent>();
        let shared_header = SharedHeader::default();
        let mut timer = Timer::new(timer_event_sender.clone(), shared_header);

        let body = move || create_default_delay_task_handler();

        let mut task_builder = TaskBuilder::default();
        task_builder
            .set_frequency(Frequency::CountDown(2, "@yearly"))
            .set_maximum_running_time(5)
            .set_task_id(1);

        // TODO: `task_builder.spawn(body)` is about 5000 ns ~ 25000ns.
        // So maintain_task takes (result of bench - task_spawn)ns.  about 1500ns.
        b.iter(|| block_on(timer.maintain_task(task_builder.spawn(body), 1, 1, 1)));
    }
}
