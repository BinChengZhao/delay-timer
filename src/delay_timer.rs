use super::{
    timer::{
        event_handle::{EventHandle},
        task::{Frequency, Task},
        timer_core::{
             Timer, TimerEvent, TimerEventSender,
            DEFAULT_TIMER_SLOT_COUNT,
        },
    },
};

use super::timer::task::TaskBuilder;
use super::utils::convenience::functions::create_default_delay_task_handler;
#[cfg(feature = "status-report")]
use super::utils::{
    convenience::functions::create_default_delay_task_handler, status_report::StatusReport,
};
use anyhow::{Context, Result};
use smol::channel::{unbounded};
use std::sync::{atomic::AtomicUsize, Arc};
use threadpool::ThreadPool;
use waitmap::WaitMap;

//FIXME: Relace Generics into Enum.
//Backupground Description :
// I have a Type Task<T> that's a smallest type. (Task<T>{task_id:i32, body:Box<dyn Fn -> T>})
// Slot<T> => Timer<T> => DelayTimer<T> all of these type base on Task<T>.
// That's the problem , I want Unified handling Task<Child>  Task<SmolTask>  in Timer
// But, If i store Task<Child> In Timer filst  I can't store Task<SmolTask> in next time,
// Because Rust compiler get T is Child so DelayTimer<T> => DelayTimer<Child>
// Then any other T instance can't store.
// So, I plan to Replace Generics into Enum.
pub struct DelayTimer {
    timer_event_sender: TimerEventSender,
}

impl Default for DelayTimer {
    fn default() -> Self {
        let mut delay_timer = DelayTimer::new();
        delay_timer.init();
        delay_timer
    }
}

//TODO:来一个hashMqp  task_id => child-handle-linklist
//可以取消任务，child-handle 可以是进程句柄 - 也可以是异步句柄， 用linklist 是因为，可能任务支持同时多个并行
impl DelayTimer {
    pub fn new() -> DelayTimer {
        let wheel_queue = EventHandle::init_task_wheel(DEFAULT_TIMER_SLOT_COUNT);
        let task_flag_map = Arc::new(WaitMap::new());
        let second_hand = Arc::new(AtomicUsize::new(0));

        //TODO: remove that.
        let (timer_event_sender, timer_event_receiver) = unbounded::<TimerEvent>();
        let mut timer = Timer::new(
            wheel_queue.clone(),
            task_flag_map.clone(),
            timer_event_sender.clone(),
        );

        //what is `ascription`.
        let mut event_handle = EventHandle::new(
            wheel_queue,
            task_flag_map,
            second_hand,
            timer_event_receiver,
        );
        // run register_features_fn

        //features include these fn:
        //TODO: timer.set_status_reporter
        //TODO: timer.set_recycle_task

        // set_recycle_task have a async_channel,
        // get task record put that in minimum heap with estimate finish time.
        // recycle task handle by a minimum heap.
        // set_recycle_task be restrain execution time 300ms.

        // Use threadpool can replenishes the pool if any worker threads panic.
        let pool = ThreadPool::new(2);

        pool.execute(move || {
            smol::block_on(async {
                timer.async_schedule().await;
            })
        });

        pool.execute(move || {
            smol::block_on(async {
                event_handle.handle_event().await;
            })
        });

        DelayTimer { timer_event_sender }
    }

    fn init(&mut self) -> Result<()> {
        self.set_recycle_task()?;
        Ok(())
    }

    pub fn set_recycle_task(&mut self) -> Result<()> {
        let mut task_builder = TaskBuilder::default();

        let body = move || {
            smol::spawn(async move {
                //TODO:bind a time-out-futures run maximum 200ms.
                ()
            })
            .detach();

            create_default_delay_task_handler()
        };

        task_builder.set_frequency(Frequency::Repeated("0/7 * * * * * *"));
        task_builder.set_task_id(0);
        let task = task_builder.spawn(body);

        self.add_task(task)
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

    pub fn add_task(&mut self, task: Task) -> Result<()> {
        self.seed_timer_event(TimerEvent::AddTask(Box::new(task)))
    }

    pub fn remove_task(&mut self, task_id: usize) -> Result<()> {
        self.seed_timer_event(TimerEvent::RemoveTask(task_id))
    }

    pub fn cancel_task(&mut self, task_id: usize, record_id: i64) -> Result<()> {
        self.seed_timer_event(TimerEvent::CancelTask(task_id, record_id))
    }

    fn seed_timer_event(&mut self, event: TimerEvent) -> Result<()> {
        self.timer_event_sender
            .try_send(event)
            .with_context(|| format!("Failed Send Event from seed_timer_event"))
    }
}
