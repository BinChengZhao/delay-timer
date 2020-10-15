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
use super::timer::{
    event_handle::EventHandle,
    task::{Task, TaskMark},
    timer_core::{Slot, Timer, TimerEvent, TimerEventSender, DEFAULT_TIMER_SLOT_COUNT},
};

use anyhow::{Context, Result};
use smol::{channel::unbounded, future::block_on};
use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc,
};
use std::time::SystemTime;
use threadpool::ThreadPool;
use waitmap::WaitMap;
//TODO:replenish the doc.
// #[cfg(feature = "status-report")]

//TODO: Set it. Motivation to move forward.
pub(crate) type SharedMotivation = Arc<AtomicBool>;
//Global sencond hand.
pub(crate) type SencondHand = Arc<AtomicU64>;
//Global Timestamp.
pub(crate) type GlobalTime = Arc<AtomicU64>;
//Shared task-wheel for operate.
pub(crate) type SharedTaskWheel = Arc<WaitMap<u64, Slot>>;
//The slot currently used for storing global tasks.
pub(crate) type SharedTaskFlagMap = Arc<WaitMap<u64, TaskMark>>;

pub struct DelayTimer {
    timer_event_sender: TimerEventSender,
}

#[derive(Clone)]
pub(crate) struct SharedHeader {
    //The task wheel has a slot dimension.
    pub(crate) wheel_queue: SharedTaskWheel,
    //Task distribution map to track where tasks are in a slot for easy removal.
    pub(crate) task_flag_map: SharedTaskFlagMap,
    //The hands of the clock.
    pub(crate) second_hand: SencondHand,
    //Global Timestamp.
    pub(crate) global_time: GlobalTime,
    //Delay_timer flag for running
    pub(crate) shared_motivation: SharedMotivation,
}

impl Default for SharedHeader {
    fn default() -> Self {
        let wheel_queue = EventHandle::init_task_wheel(DEFAULT_TIMER_SLOT_COUNT);
        let task_flag_map = Arc::new(WaitMap::new());
        let second_hand = Arc::new(AtomicU64::new(0));
        let global_time = Arc::new(AtomicU64::new(get_timestamp()));
        let shared_motivation = Arc::new(AtomicBool::new(true));

        SharedHeader {
            wheel_queue,
            task_flag_map,
            second_hand,
            global_time,
            shared_motivation,
        }
    }
}

impl Default for DelayTimer {
    fn default() -> Self {
        DelayTimer::new()
    }
}

impl DelayTimer {
    /// New a DelayTimer.
    pub fn new() -> DelayTimer {
        let shared_header = SharedHeader::default();

        //init reader sender for timer-event handle.
        let (timer_event_sender, timer_event_receiver) = unbounded::<TimerEvent>();
        let mut timer = Timer::new(timer_event_sender.clone(), shared_header.clone());

        //what is `ascription`.
        let mut event_handle = EventHandle::new(
            timer_event_receiver,
            timer_event_sender.clone(),
            shared_header,
        );

        //TODO: run register_features_fn

        // When the method finishes executing,
        // the pool has been dropped. When these two tasks finish executing,
        // the two threads will automatically release their resources after a single experience.
        let pool = ThreadPool::new(2);

        pool.execute(move || {
            smol::block_on(async {
                timer.async_schedule().await;
            })
        });

        pool.execute(move || {
            block_on(async {
                event_handle.handle_event().await;
            })
        });

        DelayTimer { timer_event_sender }
    }

    // if open "status-report", then register task 3s auto-run report
    #[cfg(feature = "status-report")]
    pub fn set_status_reporter(&mut self, status_report: impl StatusReport) -> Result<()> {
        let mut task_builder = TaskBuilder::default();

        let body = move || {
            SmolTask::spawn(async move {
                let report_result = status_report.report().await;

                if report_result.is_err() {
                    status_report.help();
                }
            })
            .detach();

            convenience::create_delay_task_handler(MyUnit)
        };

        task_builder.set_frequency(Frequency::Repeated("0/3 * * * * * *"));
        task_builder.set_task_id(0);
        let task = task_builder.spawn(body);

        self.add_task(task)
    }

    /// Add a task in timer_core by event-channel.
    pub fn add_task(&mut self, task: Task) -> Result<()> {
        self.seed_timer_event(TimerEvent::AddTask(Box::new(task)))
    }

    /// Remove a task in timer_core by event-channel.
    pub fn remove_task(&mut self, task_id: u64) -> Result<()> {
        self.seed_timer_event(TimerEvent::RemoveTask(task_id))
    }

    /// Cancel a task in timer_core by event-channel.
    pub fn cancel_task(&mut self, task_id: u64, record_id: i64) -> Result<()> {
        self.seed_timer_event(TimerEvent::CancelTask(task_id, record_id))
    }

    pub fn stop_delay_timer(&mut self) -> Result<()> {
        self.seed_timer_event(TimerEvent::StopTimer)
    }

    /// Send a event to event-handle.
    fn seed_timer_event(&mut self, event: TimerEvent) -> Result<()> {
        self.timer_event_sender
            .try_send(event)
            .with_context(|| "Failed Send Event from seed_timer_event".to_string())
    }
}

///get current OS SystemTime.
pub fn get_timestamp() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
