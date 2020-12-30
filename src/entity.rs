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
    timer_core::{Slot, Timer, TimerEvent, DEFAULT_TIMER_SLOT_COUNT},
};
use crate::prelude::*;

cfg_tokio_support!(
    use tokio::runtime::{Builder as TokioBuilder, Runtime};
    use std::sync::atomic::{AtomicUsize, Ordering};
);

use anyhow::{Context, Result};
use futures::executor::block_on;
use smol::channel::unbounded;

use snowflake::SnowflakeIdBucket;
use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc,
};
use std::thread::Builder;
use std::time::SystemTime;
use waitmap::WaitMap;

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
    #[allow(dead_code)]
    shared_header: SharedHeader,
}

#[derive(Clone)]
pub struct SharedHeader {
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
    //RuntimeInstance
    pub(crate) runtime_instance: RuntimeInstance,
    //Unique id generator.
    pub(crate) snowflakeid_bucket: SnowflakeIdBucket,
}

#[derive(Clone, Default)]
pub(crate) struct RuntimeInstance {
    //smol have no instance.
    #[cfg(feature = "tokio-support")]
    pub(crate) inner: Option<Arc<Runtime>>,
    pub(crate) kind: RuntimeKind,
}
#[derive(Clone, Debug, Copy)]
pub(crate) enum RuntimeKind {
    Smol,
    #[cfg(feature = "tokio-support")]
    Tokio,
}

impl Default for RuntimeKind {
    fn default() -> Self {
        RuntimeKind::Smol
    }
}

impl Default for SharedHeader {
    fn default() -> Self {
        let wheel_queue = EventHandle::init_task_wheel(DEFAULT_TIMER_SLOT_COUNT);
        let task_flag_map = Arc::new(WaitMap::new());
        let second_hand = Arc::new(AtomicU64::new(0));
        let global_time = Arc::new(AtomicU64::new(get_timestamp()));
        let shared_motivation = Arc::new(AtomicBool::new(true));
        let runtime_instance = RuntimeInstance::default();
        let snowflakeid_bucket = SnowflakeIdBucket::new(1, 1);

        SharedHeader {
            wheel_queue,
            task_flag_map,
            second_hand,
            global_time,
            shared_motivation,
            runtime_instance,
            snowflakeid_bucket,
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
        Self::init_by_shared_header(SharedHeader::default())
    }

    //TODO:get_status_reporter.
    // Hot-plug.

    fn init_by_shared_header(shared_header: SharedHeader) -> DelayTimer {
        //init reader sender for timer-event handle.
        let (timer_event_sender, timer_event_receiver) = unbounded::<TimerEvent>();
        let timer = Timer::new(timer_event_sender.clone(), shared_header.clone());

        let event_handle = EventHandle::new(
            timer_event_receiver,
            timer_event_sender.clone(),
            shared_header.clone(),
        );

        let runtimer_kind = shared_header.runtime_instance.kind;
        let delay_timer = DelayTimer {
            timer_event_sender,
            shared_header,
        };

        //assign task.
        match runtimer_kind {
            RuntimeKind::Smol => delay_timer.assign_task(timer, event_handle),
            #[cfg(feature = "tokio-support")]
            RuntimeKind::Tokio => delay_timer.assign_task_by_tokio(timer, event_handle),
        };

        delay_timer
    }

    fn assign_task(&self, timer: Timer, event_handle: EventHandle) {
        self.run_async_schedule(timer);
        self.run_event_handle(event_handle);
    }

    fn run_async_schedule(&self, mut timer: Timer) {
        Builder::new()
            .name("async_schedule".into())
            .spawn(move || {
                smol::block_on(async {
                    timer.async_schedule().await;
                })
            })
            .expect("async_schedule can't start.");
    }

    fn run_event_handle(&self, mut event_handle: EventHandle) {
        Builder::new()
            .name("handle_event".into())
            .spawn(move || {
                block_on(async {
                    event_handle.handle_event().await;
                })
            })
            .expect("handle_event can't start.");
    }

    cfg_status_report!();

    /// Add a task in timer_core by event-channel.
    pub fn add_task(&self, task: Task) -> Result<()> {
        self.seed_timer_event(TimerEvent::AddTask(Box::new(task)))
    }

    /// Remove a task in timer_core by event-channel.
    pub fn remove_task(&self, task_id: u64) -> Result<()> {
        self.seed_timer_event(TimerEvent::RemoveTask(task_id))
    }

    /// Cancel a task in timer_core by event-channel.
    pub fn cancel_task(&self, task_id: u64, record_id: i64) -> Result<()> {
        self.seed_timer_event(TimerEvent::CancelTask(task_id, record_id))
    }

    pub fn stop_delay_timer(&self) -> Result<()> {
        self.seed_timer_event(TimerEvent::StopTimer)
    }

    /// Send a event to event-handle.
    fn seed_timer_event(&self, event: TimerEvent) -> Result<()> {
        self.timer_event_sender
            .try_send(event)
            .with_context(|| "Failed Send Event from seed_timer_event".to_string())
    }
}

cfg_tokio_support!(
   impl DelayTimer{

    fn assign_task_by_tokio(&self, timer: Timer,event_handle: EventHandle) {
        self.run_async_schedule_by_tokio(timer);
        self.run_event_handle_by_tokio(event_handle);
    }

    fn run_async_schedule_by_tokio(&self, mut timer: Timer){
       if let Some(ref tokio_runtime_ref) = self.shared_header.runtime_instance.inner{
           let tokio_runtime = tokio_runtime_ref.clone();
        Builder::new()
        .name("async_schedule_tokio".into())
        .spawn(move || {
            tokio_runtime.block_on(async {
                timer.async_schedule().await;
            })
        })
        .expect("async_schedule can't start.");
       }

    }

    fn run_event_handle_by_tokio(&self, mut event_handle: EventHandle){
        if let Some(ref tokio_runtime_ref) = self.shared_header.runtime_instance.inner {
            let tokio_runtime = tokio_runtime_ref.clone();
            Builder::new()
                .name("handle_event_tokio".into())
                .spawn(move || {
                    tokio_runtime.block_on(async {
                        event_handle.handle_event().await;
                    })
                })
                .expect("run_event_handle_by_tokio can't start.");
        }

     }

    pub fn new_with_tokio(rt:Option<Arc<Runtime>>) -> DelayTimer {
        let mut shared_header = SharedHeader::default();
        shared_header.register_tokio_runtime(rt);
        Self::init_by_shared_header(shared_header)
    }
   }

   impl SharedHeader {
        pub(crate) fn tokio_support() -> Option<Runtime> {
            TokioBuilder::new_multi_thread().enable_all()
                .thread_name_fn(|| {
                    static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                    let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                    format!("tokio-{}", id)
                })
                .on_thread_start(|| {
                    println!("tokio-thread started");
                })
                .build()
                .ok()
        }

        pub(crate) fn register_tokio_runtime(&mut self,mut rt:Option<Arc<Runtime>>) {

            if rt.is_none(){
              rt = Some(Arc::new(Self::tokio_support().expect("init tokioRuntime is fail.")));
            }

            self.runtime_instance.inner = rt;
            self.runtime_instance.kind = RuntimeKind::Tokio;
        }
    }


);

//TODO: Translate to english.
//usein LRU cache...
//线程全局的lru缓存，

//以后spawn，任务就只用 几百ns了。

///get current OS SystemTime.
pub fn get_timestamp() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
