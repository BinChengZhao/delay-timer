//! `StatusReporter` is to expose the necessary operational information
//! to the outside world.
use crate::prelude::*;
use std::convert::TryFrom;
use future_lite::block_on;
use once_cell::sync::Lazy;


/// Global internal status-reporter.
pub(crate) static GLOBAL_STATUS_REPORTER: Lazy<(AsyncSender<PublicEvent>, AsyncReceiver<PublicEvent>)> = Lazy::new(|| {
    smol::channel::unbounded()
});

/// # Required features
///
/// This function requires the `status-report` feature of the `delay_timer`
/// crate to be enabled.
#[derive(Debug, Clone)]
pub struct StatusReporter {
    inner: AsyncReceiver<PublicEvent>,
}

impl StatusReporter {

    /// Non-blocking get `PublicEvent` via `StatusReporter`.
    pub fn next_public_event(&self) -> Result<PublicEvent, channel::TryRecvError> {
        let event = self.inner.try_recv()?;
        Ok(event)
    }

    /// Blocking get `PublicEvent` via `StatusReporter`.
    pub fn next_public_event_with_wait(&self) -> Result<PublicEvent, channel::RecvError> {
        block_on(self.inner.recv())
    }

    /// Async get `PublicEvent` via `StatusReporter`.
    pub async fn next_public_event_with_async_wait(&self) -> Result<PublicEvent, channel::RecvError> {
        Ok(self.inner.recv().await?)
    }

    pub(crate) fn new(inner: AsyncReceiver<PublicEvent>) -> Self {
        Self { inner }
    }
}

// Define types independently to avoid coupling internal types.
/// The information generated when completing a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicFinishTaskBody {
    pub(crate) task_id: u64,
    pub(crate) record_id: i64,
    pub(crate) finish_time: u64,
    pub(crate) finish_output: Option<PublicFinishOutput>,
}

// Define types independently to avoid coupling internal types.
/// The output generated when the task is completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicFinishOutput {
    /// The output generated when the process task is completed.
    ProcessOutput(std::process::Output),
    /// Exception output for a task that did not run successfully.
    ExceptionOutput(String),
}

impl PublicFinishTaskBody{

    /// Get the TaskInstance task-id.
    #[inline(always)]
    pub fn get_task_id(&self) -> u64{
        self.task_id
    }


    /// Get the TaskInstance record-id.
    #[inline(always)]
    pub fn get_record_id(&self) -> i64{
        self.record_id
    }


    /// Get the TaskInstance finish-time.
    #[inline(always)]
    pub fn get_finish_time(&self) -> u64{
        self.finish_time
    }

    /// Get the output on internal completion.
    #[inline(always)]
    pub fn get_finish_output(&mut self) -> Option<PublicFinishOutput>{
        self.finish_output.take()
    }
}

impl From<FinishTaskBody> for PublicFinishTaskBody{
    fn from(value: FinishTaskBody) -> Self{
        PublicFinishTaskBody{
            task_id:value.task_id,
            record_id:value.record_id,
            finish_time:value.finish_time,
            finish_output:value.finish_output.map(|o|o.into()),
        }
    }
}

impl From<FinishOutput> for PublicFinishOutput{
    fn from(value:FinishOutput) -> Self{
        match value{
            FinishOutput::ProcessOutput(o) => PublicFinishOutput::ProcessOutput(o),
            FinishOutput::ExceptionOutput(o) => PublicFinishOutput::ExceptionOutput(o) 
        }
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
    FinishTask(PublicFinishTaskBody),
    /// Describe which task instance timeout .
    TimeoutTask(u64, i64),
}

impl TryFrom<&TimerEvent> for PublicEvent {
    type Error = anyhow::Error;

    fn try_from(timer_event: &TimerEvent) -> Result<Self, Self::Error> {
        match timer_event {
            TimerEvent::RemoveTask(task_id) => Ok(PublicEvent::RemoveTask(*task_id)),
            TimerEvent::AppendTaskHandle(_, delay_task_handler_box) => {
                Ok(PublicEvent::RunningTask(delay_task_handler_box.get_task_id(), delay_task_handler_box.get_record_id()))
            }
            TimerEvent::FinishTask(finish_task_body) => {
                // TODO: Be wary, clone can involve a lot of memory and consume performance.
                Ok(PublicEvent::FinishTask(finish_task_body.clone().into()))
            }

            TimerEvent::TimeoutTask(task_id, record_id) => {
                Ok(PublicEvent::TimeoutTask(*task_id, *record_id))
            }

            _ => Err(anyhow!("PublicEvent only accepts timer_event some variant( RemoveTask, CancelTask ,FinishTask )!")),
        }
    }
}

impl TryFrom<TimerEvent> for PublicEvent {
    type Error = anyhow::Error;

    fn try_from(timer_event: TimerEvent) -> Result<Self, Self::Error> {
        match timer_event {
            TimerEvent::RemoveTask(task_id) => Ok(PublicEvent::RemoveTask(task_id)),
            TimerEvent::AppendTaskHandle(_, delay_task_handler_box) => {
                Ok(PublicEvent::RunningTask(delay_task_handler_box.get_task_id(), delay_task_handler_box.get_record_id()))
            }
            TimerEvent::FinishTask(finish_task_body) => {
                // TODO: Be wary, clone can involve a lot of memory and consume performance.
                Ok(PublicEvent::FinishTask(finish_task_body.into()))
            }

            TimerEvent::TimeoutTask(task_id, record_id) => {
                Ok(PublicEvent::TimeoutTask(task_id, record_id))
            }

            _ => Err(anyhow!("PublicEvent only accepts timer_event some variant( RemoveTask, CancelTask ,FinishTask )!")),
        }
    }
}

impl PublicEvent {
    /// Get the task_id corresponding to the event.
   pub fn get_task_id(&self) -> u64 {
        match self {
            PublicEvent::RemoveTask(ref task_id) => *task_id,
            PublicEvent::RunningTask(ref task_id, _) => *task_id,
            PublicEvent::FinishTask(PublicFinishTaskBody{task_id,..}) => *task_id,
            PublicEvent::TimeoutTask(ref task_id, _) => *task_id,
        }
    }

    /// Get the record_id corresponding to the event.
   pub fn get_record_id(&self) -> Option<i64> {
        match self {
            PublicEvent::RemoveTask(_) => None,
            PublicEvent::RunningTask(_,ref record_id) => Some(*record_id),
            PublicEvent::FinishTask(PublicFinishTaskBody{record_id,..}) => Some(*record_id),
            PublicEvent::TimeoutTask(_,ref record_id) => Some(*record_id),
      
        }
    }
}
