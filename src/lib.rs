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
//!
//! 除了简单的没几秒执行一次

//!，还可以指定 特定日期，比如周日的凌晨4点执行一个备份任务

//!还支持判断前一个任务没执行完成时，是否并行，最大并行数量

//!同步异步任务都支持， 支持执行过程中取消

//!支持 smol tokio async-std构建的Future等
//!
#![feature(split_inclusive)]
#![feature(linked_list_cursors)]
#![feature(test)]
#![feature(associated_type_defaults)]
//TODO:When the version is stable in the future, we should consider using stable compile unified.
extern crate test;

pub(crate) use utils::parse::shell_command::{ChildGuard, ChildGuardList};

pub mod delay_timer;
#[macro_use]
pub mod macros;
pub mod timer;
pub mod utils;

pub use anyhow::Result as AnyResult;
pub use cron_clock;
pub use macros::*;
pub use smol::future as future_lite;
pub use smol::spawn as async_spawn;
pub use smol::unblock as unblock_spawn;
pub use timer::runtime_trace::task_handle::DelayTaskHandler;
pub use timer::task::{Frequency, Task, TaskBuilder};
pub use utils::convenience::cron_expression_grammatical_candy::CronCandy;

cfg_tokio_support!(
    pub use tokio::task::spawn as tokio_async_spawn;
    pub use tokio::task::spawn_blocking as tokio_unblock_spawn;
    pub(crate) use tokio::sync::mpsc::{
        UnboundedReceiver as AsyncReceiver, UnboundedSender as AsyncSender,
        unbounded_channel as async_unbounded_channel,
    };
    pub(crate) use tokio::sync::Mutex as AsyncMutex;
    pub(crate) use tokio::task::yield_now;
);

cfg_not_tokio_support!(
    pub(crate) use smol::channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
    pub(crate) use smol::lock::Mutex as AsyncMutex;
    pub(crate) use smol::future::yield_now;
    pub(crate) use smol::{channel::unbounded as async_unbounded_channel, future::block_on};
);

//TODO: Maybe can independent bench mod to one project.
//Or via `rustversion` Isolation of different modules.
//TODO: Add a prelude.
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

    //TODO:可以独立benches  到一个跟examples平级的目录，参考 async-task
    #[bench]
    fn bench_task_spwan(b: &mut Bencher) {
        let body = move || create_default_delay_task_handler();

        let mut task_builder = TaskBuilder::default();
        task_builder
            .set_frequency(Frequency::CountDown(1, "@yearly"))
            .set_maximum_running_time(5)
            .set_task_id(1);

        // String parsing to corn-expression -> iterator is the most time-consuming operation about 1500ns ~ 3500 ns.
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
        b.iter(|| block_on(timer.maintain_task(task_builder.spawn(body).unwrap(), 1, 1, 1)));
    }
}
