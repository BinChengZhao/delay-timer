#![feature(test)]
#![deny(warnings)]

extern crate test;

use delay_timer::entity::SharedHeader;
use delay_timer::prelude::*;
use delay_timer::timer::timer_core::{Timer, TimerEvent};

use futures::executor::block_on;
use smol::channel::unbounded;
use test::Bencher;

#[bench]
fn bench_task_spwan(b: &mut Bencher) {
    let body = move |_| create_default_delay_task_handler();

    let mut task_builder = TaskBuilder::default();
    task_builder
        .set_frequency(Frequency::CountDown(1, "@yearly"))
        .set_maximum_running_time(5)
        .set_task_id(1);

    // String parsing to corn-expression -> iterator is the most time-consuming operation about 1500ns ~ 3500 ns.
    // The iterator is used to find out when the next execution should take place, in about 500 ns.
    b.iter(|| task_builder.spawn_async_routine(body.clone()));
}

#[bench]
fn bench_maintain_task(b: &mut Bencher) {
    let (timer_event_sender, _timer_event_receiver) = unbounded::<TimerEvent>();
    let shared_header = SharedHeader::default();
    let mut timer = Timer::new(timer_event_sender.clone(), shared_header);

    let body = move |_| create_default_delay_task_handler();

    let mut task_builder = TaskBuilder::default();
    task_builder
        .set_frequency(Frequency::CountDown(2, "@yearly"))
        .set_maximum_running_time(5)
        .set_task_id(1);

    // `task_builder.spawn_async_routine(body)` is about 1500 ns .
    // So maintain_task takes (result of bench - task_spawn)ns.  about 1000ns.
    b.iter(|| {
        block_on({
            async {
                if let Ok(task) = task_builder.spawn_async_routine(body) {
                    timer.maintain_task(task, 1, 1).await.ok();
                }
            }
        })
    });
}

#[bench]
fn bench_try_wait(b: &mut Bencher) {
    use std::process::Command;

    if let Ok(mut child) = Command::new("ps").spawn_async_routine() {
        b.iter(|| child.try_wait());
    }
}

#[bench]
fn bench_timestamp(b: &mut Bencher) {
    b.iter(|| timestamp());
}
