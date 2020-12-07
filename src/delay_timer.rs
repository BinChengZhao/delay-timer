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
use crate::prelude::*;

cfg_tokio_support!(
    use tokio::runtime::{Builder as TokioBuilder, Runtime};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::mpsc::unbounded_channel;
);

cfg_status_report!(
    use crate::utils::{StatusReport, functions::create_default_delay_task_handler};
    use crate::{TaskBuilder, Frequency, async_spawn};
);

use anyhow::{Context, Result};
//FIXME: not AND any "tokio-support" or "tokio-xxx"
cfg_smol_support!(
    use futures::executor::block_on;
    use smol::channel::unbounded;
);

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
    shared_header: SharedHeader,
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
    //OtherRuntimes
    pub(crate) other_runtimes: OtherRuntimes,
}

#[derive(Clone, Default)]
pub(crate) struct OtherRuntimes {
    #[cfg(feature = "tokio-support")]
    pub(crate) tokio: Option<Arc<Runtime>>,
}

impl Default for SharedHeader {
    fn default() -> Self {
        let wheel_queue = EventHandle::init_task_wheel(DEFAULT_TIMER_SLOT_COUNT);
        let task_flag_map = Arc::new(WaitMap::new());
        let second_hand = Arc::new(AtomicU64::new(0));
        let global_time = Arc::new(AtomicU64::new(get_timestamp()));
        let shared_motivation = Arc::new(AtomicBool::new(true));
        let other_runtimes = OtherRuntimes::default();

        SharedHeader {
            wheel_queue,
            task_flag_map,
            second_hand,
            global_time,
            shared_motivation,
            other_runtimes,
        }
    }
}

impl Default for DelayTimer {
    fn default() -> Self {
        DelayTimer::new()
    }
}

//TODO:支持tokio，我只启动一个runtime，跟暴露几个API
impl DelayTimer {
    /// New a DelayTimer.
    pub fn new() -> DelayTimer {
        Self::init_by_shared_header(SharedHeader::default())
    }

    #[cfg(not(feature = "tokio-support"))]
    fn timer_event_channel() -> (AsyncSender<TimerEvent>, AsyncReceiver<TimerEvent>) {
        unbounded::<TimerEvent>()
    }

    fn init_by_shared_header(shared_header: SharedHeader) -> DelayTimer {
        //init reader sender for timer-event handle.
        let (timer_event_sender, timer_event_receiver) = Self::timer_event_channel();
        let timer = Timer::new(timer_event_sender.clone(), shared_header.clone());

        //what is `ascription`.
        let event_handle = EventHandle::new(
            timer_event_receiver,
            timer_event_sender.clone(),
            shared_header.clone(),
        );

        let delay_timer = DelayTimer {
            timer_event_sender,
            shared_header,
        };
        //assign task.
        delay_timer.assign_task(timer, event_handle);

        delay_timer
    }

    cfg_smol_support!(
        fn assign_task(&self, timer: Timer, mut event_handle: EventHandle) {
            self.run_async_schedule(timer);
    
            Builder::new()
                .name("handle_event".into())
                .spawn(move || {
                    block_on(async {
                        event_handle.handle_event().await;
                    })
                })
                .expect("handle_event can't start.");
        }
    );

    cfg_tokio_support!(
        fn assign_task(&self, timer: Timer, mut event_handle: EventHandle) {
            self.run_async_schedule(timer);

            if let Some(ref tokio_runtime_ref) = self.shared_header.other_runtimes.tokio {
                let tokio_runtime = tokio_runtime_ref.clone();
                Builder::new()
                    .name("handle_event_tokio".into())
                    .spawn(move || {
                        tokio_runtime.block_on(async {
                            event_handle.handle_event().await;
                        })
                    })
                    .expect("async_schedule can't start.");
            }
        }
    );

    #[cfg(not(feature = "tokio-support"))]
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

    cfg_status_report!(
        // if open "status-report", then register task 3s auto-run report
        //TODO: needs run in runtime.
        pub fn set_status_reporter(&self, status_report: impl StatusReport) -> Result<()> {
            let mut task_builder = TaskBuilder::default();
            let status_report_arc = Arc::new(status_report);

            let body = move || {
                let status_report_ref = status_report_arc.clone();
                async_spawn(async move {
                    let report_result = status_report_ref.report(None).await;

                    // if report_result.is_err() {
                    //     status_report.help();
                    // }
                })
                .detach();

                create_default_delay_task_handler()
            };

            task_builder.set_frequency(Frequency::Repeated("0/3 * * * * * *"));
            //single use.
            task_builder.set_task_id(0);
            let task = task_builder.spawn(body).unwrap();

            self.add_task(task)
        }
    );

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
    #[cfg(not(feature = "tokio-support"))]
    fn seed_timer_event(&self, event: TimerEvent) -> Result<()> {
        self.timer_event_sender
            .try_send(event)
            .with_context(|| "Failed Send Event from seed_timer_event".to_string())
    }
}

//TODO:笔记
//条件编译的 cfg 标记是以item为单位的，可以理解为一个函数单元
//不能在函数内部，一半用cfg标记，一半不用
//如果需要根据不同 cfg 标记调用不同的 statement 单元，用方法包住
//让用户选择去调用。

cfg_tokio_support!(
   impl DelayTimer{
    fn seed_timer_event(&self, event: TimerEvent) -> Result<()> {
        self.timer_event_sender
            .send(event)
            .with_context(|| "Failed Send Event from seed_timer_event".to_string())
    }

    fn timer_event_channel() -> (AsyncSender<TimerEvent>, AsyncReceiver<TimerEvent>) {
        unbounded_channel::<TimerEvent>()
    }

    fn run_async_schedule(&self, mut timer: Timer){
       if let Some(ref tokio_runtime_ref) = self.shared_header.other_runtimes.tokio{
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

            self.other_runtimes.tokio = rt;
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
