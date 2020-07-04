use super::timer::{
    task::Task,
    timer::{Timer, TimerEvent, TimerEventReceiver, TimerEventSender},
};
use futures::future;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

//TODO:结构体的内部字段，命名一致都用小写下划线
pub struct DelayTimer {
    timer_event_sender: TimerEventSender,
}

impl DelayTimer {
    pub fn new() -> DelayTimer {
        let (timer_event_sender, timer_event_receiver) = channel::<TimerEvent>();
        let mut timer = Timer::new(timer_event_receiver);
        //TODO: Hidden init;
        timer.init();

        thread::spawn(|| smol::run(future::pending::<()>()));
        let a: Box<dyn Fn() -> i32 + Send + Sync> = Box::new(|| 1 + 1);
        thread::spawn(move || timer.schedule());

        DelayTimer { timer_event_sender }
    }

    pub fn add_task(&mut self, task: Task) {
        self.seed_timer_event(TimerEvent::AddTask(task));
    }

    pub fn remove_task(&mut self, task_id: u32) {
        self.seed_timer_event(TimerEvent::RemoveTask(task_id));
    }

    pub fn cancel_task(&mut self, task_id: u32) {
        self.seed_timer_event(TimerEvent::CancelTask(task_id));
    }

    fn seed_timer_event(&mut self, event: TimerEvent) {
        //TODO: handle error here;
        self.timer_event_sender.send(event);
    }
}
