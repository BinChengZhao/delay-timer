use super::timer::{
    task::Task,
    timer_core::{Timer, TimerEvent, TimerEventSender},
};
use anyhow::{Context, Result};
use std::{
    collections::{HashMap, LinkedList},
    sync::mpsc::channel,
};
use threadpool::ThreadPool;

pub struct DelayTimer {
    timer_event_sender: TimerEventSender,
}

//TaskTrace-全局的句柄
//当进程消亡，跟异步任务drop的时候对应的链表也减少，如果没值则删除k/v
struct TaskTrace<T: DelayTimerTask> {
    inner: HashMap<u32, LinkedList<T>>,
}

//取消都是走异步的
trait DelayTimerTask {
    fn cancel(self);
}

//TODO:来一个hashMqp  task_id => child-handle-linklist
//可以取消任务，child-handle 可以是进程句柄 - 也可以是异步句柄， 用linklist 是因为，可能任务支持同时多个并行
impl DelayTimer {
    pub fn new() -> DelayTimer {
        let (timer_event_sender, timer_event_receiver) = channel::<TimerEvent>();
        let mut timer = Timer::new(timer_event_receiver);

        // Use threadpool can replenishes the pool if any worker threads panic.
        let pool = ThreadPool::new(1);

        //sync schedule
        // thread::spawn(move || timer.schedule());

        pool.execute(move || {
            smol::run(async {
                timer.async_schedule().await;
            })
        });

        DelayTimer { timer_event_sender }
    }

    pub fn add_task(&mut self, task: Task) {
        self.seed_timer_event(TimerEvent::AddTask(Box::new(task)));
    }

    pub fn remove_task(&mut self, task_id: u32) {
        self.seed_timer_event(TimerEvent::RemoveTask(task_id));
    }

    pub fn cancel_task(&mut self, task_id: u32) {
        self.seed_timer_event(TimerEvent::CancelTask(task_id));
    }

    fn seed_timer_event(&mut self, event: TimerEvent) -> Result<()> {
        self.timer_event_sender
            .send(event)
            .with_context(|| format!("Failed Send Event from seed_timer_event"))
    }
}

impl Default for DelayTimer {
    fn default() -> Self {
        Self::new()
    }
}
