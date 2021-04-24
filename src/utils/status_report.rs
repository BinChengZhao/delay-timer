//! `StatusReporter` is to expose the necessary operational information
//! to the outside world.
use crate::prelude::*;
use std::convert::TryFrom;

/// # Required features
///
/// This function requires the `status-report` feature of the `delay_timer`
/// crate to be enabled.
#[derive(Debug, Clone)]
pub struct StatusReporter {
    inner: AsyncReceiver<PublicEvent>,
}

impl StatusReporter {

    /// Get `PublicEvent` via `StatusReporter`.
    pub fn get_public_event(&self) -> AnyResult<PublicEvent> {
        let event = self.inner.try_recv()?;
        Ok(event)
    }

    pub(crate) fn new(inner: AsyncReceiver<PublicEvent>) -> Self {
        Self { inner }
    }
}

/// `PublicEvent`, describes the open events that occur in the delay-timer of the task.
#[derive(Debug, Clone)]
pub enum PublicEvent {
    /// Describes which task is removed.
    RemoveTask(u64),
    /// Describes which task produced a new running instance, record the id.
    RunningTask(u64, i64),
    /// Describe which task instance completed.
    FinishTask(FinishTaskBody),
    /// Describe which task instance timeout .
    TimeoutTask(u64, i64),
}

impl TryFrom<&TimerEvent> for PublicEvent {
    type Error = &'static str;

    fn try_from(timer_event: &TimerEvent) -> Result<Self, Self::Error> {
        match timer_event {
            TimerEvent::RemoveTask(task_id) => Ok(PublicEvent::RemoveTask(*task_id)),
            TimerEvent::AppendTaskHandle(_, delay_task_handler_box) => {
                Ok(PublicEvent::RunningTask(delay_task_handler_box.get_task_id(), delay_task_handler_box.get_record_id()))
            }
            TimerEvent::FinishTask(finish_task_body) => {
                // TODO: Be wary, clone can involve a lot of memory and consume performance.
                Ok(PublicEvent::FinishTask(finish_task_body.clone()))
            }

            TimerEvent::TimeoutTask(task_id, record_id) => {
                Ok(PublicEvent::TimeoutTask(*task_id, *record_id))
            }

            _ => Err("PublicEvent only accepts timer_event some variant( RemoveTask, CancelTask ,FinishTask )!"),
        }
    }
}

impl PublicEvent {
    /// Get the task_id corresponding to the event.
   pub fn get_task_id(&self) -> u64 {
        match self {
            PublicEvent::RemoveTask(ref task_id) => *task_id,
            PublicEvent::RunningTask(ref task_id, _) => *task_id,
            PublicEvent::FinishTask(FinishTaskBody{task_id,..}) => *task_id,
            PublicEvent::TimeoutTask(ref task_id, _) => *task_id,
        }
    }

    /// Get the record_id corresponding to the event.
   pub fn get_record_id(&self) -> Option<i64> {
        match self {
            PublicEvent::RemoveTask(_) => None,
            PublicEvent::RunningTask(_,ref record_id) => Some(*record_id),
            PublicEvent::FinishTask(FinishTaskBody{record_id,..}) => Some(*record_id),
            PublicEvent::TimeoutTask(_,ref record_id) => Some(*record_id),
      
        }
    }
}
