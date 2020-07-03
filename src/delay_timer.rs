use super::timer::{
    task::Task,
    timer::{Timer, TimerEvent, TimerEventReceiver, TimerEventSender},
};
use futures::future;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

//TODO:结构体的内部字段，命名一致都用小写下划线
struct DelayTimer {
    TimerEventSender: TimerEventSender,
}

impl DelayTimer {
    fn new() -> DelayTimer {
        let (TimerEventSender, TimerEventReceiver) = channel::<TimerEvent>();
        let mut timer = Timer::new(TimerEventReceiver);
        thread::spawn(|| smol::run(future::pending::<()>()));
        let a: Box<dyn Fn() -> i32 + Send + Sync> = Box::new(|| 1 + 1);
        thread::spawn(move || timer.schedule());

        //TODO:记录一下，通过增加Send 限制解决的编译问题
        DelayTimer { TimerEventSender }
    }

    fn add_task(&mut self, task: Task) {
        self.seed_timer_event(TimerEvent::AddTask(task));
    }

    fn remove_task(&mut self, task_id: u32) {
        self.seed_timer_event(TimerEvent::RemoveTask(task_id));
    }

    fn cancel_task(&mut self, task_id: u32) {
        self.seed_timer_event(TimerEvent::CancelTask(task_id));
    }

    fn seed_timer_event(&mut self, event: TimerEvent) {
        self.seed_timer_event(event);
    }
}
