/// `StatusReporter` is to expose the necessary operational information
/// to the outside world.
use crate::prelude::*;
use std::convert::TryFrom;

#[derive(Debug, Clone)]
pub struct StatusReporter {
    inner: AsyncReceiver<PublicEvent>,
}

impl StatusReporter {
    pub fn get_public_event(&self) -> AnyResult<PublicEvent> {
        let event = dbg!(self.inner.try_recv())?;
        Ok(event)
    }

    pub(crate) fn new(inner: AsyncReceiver<PublicEvent>) -> Self {
        Self { inner }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PublicEvent {
    RemoveTask(u64),
    RunningTask(u64, i64),
    FinishTask(u64, i64),
}

impl TryFrom<&TimerEvent> for PublicEvent {
    type Error = &'static str;

    fn try_from(timer_event: &TimerEvent) -> Result<Self, Self::Error> {
        match timer_event {
            TimerEvent::RemoveTask(task_id) => Ok(PublicEvent::RemoveTask(*task_id)),
            TimerEvent::AppendTaskHandle(_, delay_task_handler_box) => {
                Ok(PublicEvent::RunningTask(delay_task_handler_box.get_task_id(), delay_task_handler_box.get_record_id()))
            }
            TimerEvent::FinishTask(task_id, record_id) => {
                Ok(PublicEvent::FinishTask(*task_id, *record_id))
            }
            _ => Err("PublicEvent only accepts timer_event some variant( RemoveTask, CancelTask ,FinishTask )!"),
        }
    }
}

impl PublicEvent {
   pub fn get_task_id(&self) -> u64 {
        match self {
            PublicEvent::RemoveTask(ref task_id) => *task_id,
            PublicEvent::RunningTask(ref task_id, _) => *task_id,
            PublicEvent::FinishTask(ref task_id, _) => *task_id,
        }
    }

   pub fn get_record_id(&self) -> Option<i64> {
        match self {
            PublicEvent::RemoveTask(_) => None,
            PublicEvent::RunningTask(_,ref record_id) => Some(*record_id),
            PublicEvent::FinishTask(_,ref record_id) => Some(*record_id),
        }
    }
}
