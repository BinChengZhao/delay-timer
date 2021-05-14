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
    event_handle::{EventHandle, EventHandleBuilder},
    task::{Task, TaskMark},
    timer_core::{Timer, TimerEvent, DEFAULT_TIMER_SLOT_COUNT},
    Slot,
};
use crate::prelude::*;
use crate::timer::runtime_trace::task_instance::task_instance_chain_pair;

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use std::thread::Builder;
use std::time::SystemTime;

use anyhow::{Context, Result};
use futures::executor::block_on;
use smol::channel::unbounded;
use snowflake::{SnowflakeIdGenerator};

cfg_tokio_support!(
    use tokio::runtime::{Builder as TokioBuilder, Runtime};
    use std::sync::atomic::{AtomicUsize, Ordering};
);

cfg_status_report!(
    use crate::utils::status_report::StatusReporter;
    use anyhow::anyhow;
);

// Set it. Motivation to move forward.
pub(crate) type SharedMotivation = Arc<AtomicBool>;
// Global sencond hand.
pub(crate) type SencondHand = Arc<AtomicU64>;
// Global Timestamp.
pub(crate) type GlobalTime = Arc<AtomicU64>;
// Shared task-wheel for operate.
pub(crate) type SharedTaskWheel = Arc<DashMap<u64, Slot>>;
// The slot currently used for storing global tasks.
pub(crate) type SharedTaskFlagMap = Arc<DashMap<u64, TaskMark>>;

// FIXME: Set `machine_id` and `node_id` to `SnowflakeIdGenerator` when initialized by default of DelayTimerBuilder.
/// Builds DelayTimer with custom configuration values.
///
/// Methods can be chained in order to set the configuration values. The
/// DelayTimer is constructed by calling `build`.
///
/// # Examples
///
/// ```
/// use delay_timer::entity::DelayTimerBuilder;
///
///
/// ```
#[derive(Clone, Debug, Default)]
pub struct DelayTimerBuilder {
    shared_header: SharedHeader,
    timer_event_channel: Option<(AsyncSender<TimerEvent>, AsyncReceiver<TimerEvent>)>,
    /// Whether or not to enable the status-report
    #[cfg(feature = "status-report")]
    enable_status_report: bool,
    #[cfg(feature = "status-report")]
    status_report_channel: Option<(AsyncSender<PublicEvent>, AsyncReceiver<PublicEvent>)>,
}

/// DelayTimer is an abstraction layer that helps users solve execution cycle synchronous/asynchronous tasks.
#[derive(Clone, Debug)]
pub struct DelayTimer {
    #[allow(dead_code)]
    shared_header: SharedHeader,
    timer_event_sender: TimerEventSender,
    #[cfg(feature = "status-report")]
    status_reporter: Option<StatusReporter>,
}

/// SharedHeader Store the core context of the runtime.
#[derive(Clone)]
pub struct SharedHeader {
    // The task wheel has a slot dimension.
    pub(crate) wheel_queue: SharedTaskWheel,
    // Task distribution map to track where tasks are in a slot for easy removal.
    pub(crate) task_flag_map: SharedTaskFlagMap,
    // The hands of the clock.
    pub(crate) second_hand: SencondHand,
    // Global Timestamp.
    pub(crate) global_time: GlobalTime,
    // Delay_timer flag for running
    pub(crate) shared_motivation: SharedMotivation,
    // RuntimeInstance
    pub(crate) runtime_instance: RuntimeInstance,
    // Unique id generator.
    // TODO: Arc<SnowflakeIdGenerator> or update machine_id node_id by Event.
    pub(crate) snowflakeid_generator: SnowflakeIdGenerator,
}

impl fmt::Debug for SharedHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("")
            .field(&self.second_hand)
            .field(&self.global_time)
            .field(&self.shared_motivation)
            .field(&self.runtime_instance)
            .field(&self.snowflakeid_generator)
            .finish()
    }
}

#[derive(Clone, Default, Debug)]
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
        let task_flag_map = Arc::new(DashMap::new());
        let second_hand = Arc::new(AtomicU64::new(0));
        let global_time = Arc::new(AtomicU64::new(get_timestamp()));
        let shared_motivation = Arc::new(AtomicBool::new(true));
        let runtime_instance = RuntimeInstance::default();
        let snowflakeid_generator = SnowflakeIdGenerator::new(1, 1);

        SharedHeader {
            wheel_queue,
            task_flag_map,
            second_hand,
            global_time,
            shared_motivation,
            runtime_instance,
            snowflakeid_generator,
        }
    }
}

impl Default for DelayTimer {
    fn default() -> Self {
        DelayTimerBuilder::default().build()
    }
}

impl DelayTimerBuilder {
    /// Build DelayTimer.
    pub fn build(mut self) -> DelayTimer {
        self.lauch();
        self.init_delay_timer()
    }

    // Start the DelayTimer.
    fn lauch(&mut self) {
        let mut event_handle_builder = EventHandleBuilder::default();
        event_handle_builder
            .timer_event_receiver(self.get_timer_event_receiver())
            .timer_event_sender(self.get_timer_event_sender())
            .shared_header(self.shared_header.clone());

        #[cfg(feature = "status-report")]
        if self.enable_status_report {
            event_handle_builder.status_report_sender(self.get_status_report_sender());
        }

        let event_handle = event_handle_builder.build();

        let timer = Timer::new(self.get_timer_event_sender(), self.shared_header.clone());

        match self.shared_header.runtime_instance.kind {
            RuntimeKind::Smol => self.assign_task(timer, event_handle),
            #[cfg(feature = "tokio-support")]
            RuntimeKind::Tokio => self.assign_task_by_tokio(timer, event_handle),
        };
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
            .name("event_handle".into())
            .spawn(move || {
                block_on(async {
                    event_handle.lauch().await;
                })
            })
            .expect("event_handle can't start.");
    }

    fn init_delay_timer(&mut self) -> DelayTimer {
        let timer_event_sender = self.get_timer_event_sender();

        let shared_header = self.shared_header.clone();

        #[cfg(feature = "status-report")]
        let mut status_reporter = None;
        #[cfg(feature = "status-report")]
        if self.enable_status_report {
            status_reporter = Some(StatusReporter::new(self.get_status_report_receiver()));
        }

        DelayTimer {
            shared_header,
            timer_event_sender,
            #[cfg(feature = "status-report")]
            status_reporter,
        }
    }

    fn get_timer_event_sender(&mut self) -> AsyncSender<TimerEvent> {
        self.timer_event_channel
            .get_or_insert_with(unbounded::<TimerEvent>)
            .0
            .clone()
    }

    fn get_timer_event_receiver(&mut self) -> AsyncReceiver<TimerEvent> {
        self.timer_event_channel
            .get_or_insert_with(unbounded::<TimerEvent>)
            .1
            .clone()
    }
}

impl DelayTimer {
    /// New a DelayTimer.
    pub fn new() -> DelayTimer {
        DelayTimerBuilder::default().build()
    }

    /// Add a task in timer_core by event-channel.
    pub fn add_task(&self, task: Task) -> Result<()> {
        self.seed_timer_event(TimerEvent::AddTask(Box::new(task)))
    }

    /// Add a task in timer_core by event-channel.
    /// But it will return a handle that can constantly take out new instances of the task.
    pub fn insert_task(&self, task: Task) -> Result<TaskInstancesChain> {
        let (mut task_instances_chain, task_instances_chain_maintainer) =
            task_instance_chain_pair();
        task_instances_chain.timer_event_sender = Some(self.timer_event_sender.clone());

        self.seed_timer_event(TimerEvent::InsertTask(
            Box::new(task),
            task_instances_chain_maintainer,
        ))?;
        Ok(task_instances_chain)
    }

    /// Update a task in timer_core by event-channel.
    pub fn update_task(&self, task: Task) -> Result<()> {
        self.seed_timer_event(TimerEvent::UpdateTask(Box::new(task)))
    }

    /// Remove a task in timer_core by event-channel.
    pub fn remove_task(&self, task_id: u64) -> Result<()> {
        self.seed_timer_event(TimerEvent::RemoveTask(task_id))
    }

    /// Cancel a task in timer_core by event-channel.
    /// `Cancel` is for instances derived from the task running up.
    pub fn cancel_task(&self, task_id: u64, record_id: i64) -> Result<()> {
        self.seed_timer_event(TimerEvent::CancelTask(task_id, record_id))
    }

    /// Stop DelayTimer, running tasks are not affected.
    pub fn stop_delay_timer(&self) -> Result<()> {
        self.seed_timer_event(TimerEvent::StopTimer)
    }

    // pub fn update_snowflakeid_generator_conf(){

    // }

    /// Send a event to event-handle.
    fn seed_timer_event(&self, event: TimerEvent) -> Result<()> {
        self.timer_event_sender
            .try_send(event)
            .with_context(|| "Failed Send Event from seed_timer_event".to_string())
    }
}

cfg_tokio_support!(
/// # Required features
///
/// This function requires the `tokio-support` feature of the `delay_timer`
/// crate to be enabled.
   impl DelayTimerBuilder{

    fn assign_task_by_tokio(&mut self, timer: Timer,event_handle: EventHandle) {
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
                .name("event_handle_tokio".into())
                .spawn(move || {
                    tokio_runtime.block_on(async {
                        event_handle.lauch().await;
                    })
                })
                .expect("event_handle_handle_by_tokio can't start.");
        }

     }

     /// With this API, let DelayTimer internally use the user custom TokioRuntime.
     /// If None is given, a custom TokioRuntime is generated internally.
    pub fn tokio_runtime(mut self, rt:Option<Arc<Runtime>>) ->Self {
        self.shared_header.register_tokio_runtime(rt);
        self
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
                    info!("tokio-thread started");
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

cfg_status_report!(
/// # Required features
///
/// This function requires the `status-report` feature of the `delay_timer`
/// crate to be enabled.
    impl DelayTimerBuilder {

        /// Whether to expose public events.
        pub fn enable_status_report(mut self) -> Self {
            self.enable_status_report = true;
            self
        }

        fn get_status_report_sender(&mut self) -> AsyncSender<PublicEvent> {
            self.status_report_channel
                .get_or_insert_with(unbounded::<PublicEvent>)
                .0
                .clone()
        }

        fn get_status_report_receiver(&mut self) -> AsyncReceiver<PublicEvent> {
            self.status_report_channel
                .get_or_insert_with(unbounded::<PublicEvent>)
                .1
                .clone()
        }
    }

    impl DelayTimer {

        /// Take StatusReporter from DelayTimer, through which you can get public events.
        pub fn take_status_reporter(&mut self) -> Option<StatusReporter> {
            self.status_reporter.take()
        }

        /// Access to public events through DelayTimer.
        pub fn get_public_event(&self) -> anyhow::Result<PublicEvent> {
            if let Some(status_reporter) = self.status_reporter.as_ref() {
                return status_reporter.next_public_event();
            }

            Err(anyhow!("Have no status-reporter."))
        }
    }

);

//TODO: Since the system clock may be adjusted,
// an internal time should be maintained
// to get rid of system interference,
// and this change can also be applied to snowflake-rs.
/// get current OS SystemTime.
pub fn get_timestamp() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}

//TODO: Since the system clock may be adjusted,
// an internal time should be maintained
// to get rid of system interference,
// and this change can also be applied to snowflake-rs.
/// get current OS SystemTime.
pub fn get_timestamp_micros() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_micros(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
