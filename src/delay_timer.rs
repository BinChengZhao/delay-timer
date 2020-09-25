use super::timer::{
    event_handle::EventHandle,
    task::Task,
    timer_core::{Timer, TimerEvent, TimerEventSender, DEFAULT_TIMER_SLOT_COUNT},
};

use anyhow::{Context, Result};
use smol::channel::unbounded;
use std::sync::{atomic::AtomicU64, Arc};
use threadpool::ThreadPool;
use waitmap::WaitMap;

// #[cfg(feature = "status-report")]

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

impl DelayTimer {
    pub fn new() -> DelayTimer {
        let wheel_queue = EventHandle::init_task_wheel(DEFAULT_TIMER_SLOT_COUNT);
        let task_flag_map = Arc::new(WaitMap::new());
        let second_hand = Arc::new(AtomicU64::new(0));

        //init reader sender for timer-event handle.
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
            timer_event_sender.clone(),
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
        //TODO: Maybe can use easy-parallel.
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
        Ok(())
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

    pub fn remove_task(&mut self, task_id: u64) -> Result<()> {
        self.seed_timer_event(TimerEvent::RemoveTask(task_id))
    }

    pub fn cancel_task(&mut self, task_id: u64, record_id: i64) -> Result<()> {
        self.seed_timer_event(TimerEvent::CancelTask(task_id, record_id))
    }

    fn seed_timer_event(&mut self, event: TimerEvent) -> Result<()> {
        self.timer_event_sender
            .try_send(event)
            .with_context(|| "Failed Send Event from seed_timer_event".to_string())
    }
}
