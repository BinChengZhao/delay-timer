// status_report is mod  for report node heathy
// if open feature status-report, then compile that mod .
// mapping
use crate::prelude::*;
use std::convert::TryFrom;

#[derive(Debug, Clone)]
struct StatusReport {
    inner: AsyncReceiver<PublicEvent>,
}

impl StatusReport {
    pub fn get_public_event(&self) -> AnyResult<PublicEvent> {
        let event = self.inner.try_recv()?;
        Ok(event)
    }
}

#[derive(Debug, Copy, Clone)]
enum PublicEvent {
    RemoveTask(u64),
    CancelTask(u64, i64),
    FinishTask(u64, i64),
}

impl TryFrom<TimerEvent> for PublicEvent {
    type Error = &'static str;

    fn try_from(timer_event: TimerEvent) -> Result<Self, Self::Error> {
        match timer_event {
            TimerEvent::RemoveTask(task_id) => Ok(PublicEvent::RemoveTask(task_id)),
            TimerEvent::CancelTask(task_id, record_id) => {
                Ok(PublicEvent::CancelTask(task_id, record_id))
            }
            TimerEvent::FinishTask(task_id, record_id) => {
                Ok(PublicEvent::FinishTask(task_id, record_id))
            }
            _ => Err("PublicEvent only accepts timer_event some variant( RemoveTask, CancelTask ,FinishTask )!"),
        }
    }
}
