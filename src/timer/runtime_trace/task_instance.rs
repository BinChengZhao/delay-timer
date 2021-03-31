use crate::prelude::*;

use std::collections::LinkedList;
use std::convert::From;
use std::sync::{Arc, Weak};
use std::time::Duration;

use anyhow::{anyhow, Context, Result as AnyResult};
use arc_swap::ArcSwap;
use event_listener::Event;

/// instance of task running.
#[derive(Debug, Default)]
pub struct Instance {
    /// The id of task.
    task_id: u64,
    /// The id of task running record.
    record_id: i64,
    /// The event view of inner task.
    event: Event,
}
/// Chain of task run instances.
/// For User access to Running-Task's instance.
#[derive(Debug, Clone)]
pub struct TaskInstancesChain {
    pub(crate) inner: Arc<ArcSwap<LinkedList<Arc<Instance>>>>,
}

/// Chain of task run instances.
/// For inner maintain to Running-Task's instance.
#[derive(Debug, Clone, Default)]
pub struct TaskInstancesChainMaintainer {
    pub(crate) inner: Weak<ArcSwap<LinkedList<Arc<Instance>>>>,
}

impl Instance {
    pub(crate) fn set_task_id(mut self, task_id: u64) -> Instance {
        self.task_id = task_id;
        self
    }

    pub(crate) fn set_record_id(mut self, record_id: i64) -> Instance {
        self.record_id = record_id;
        self
    }

    /// Cancel the currently running task instance and block the thread to wait.
    pub fn cancel_and_wait(&self) -> AnyResult<()> {
        self.cancel()?;

        self.event.listen().wait();
        Ok(())
    }

    /// Cancel the currently running task instance and block the thread to wait
    /// for an expected amount of time.
    pub fn cancel_and_wait_timeout(&self, timeout: Duration) -> AnyResult<()> {
        self.cancel()?;

        self.event
            .listen()
            .wait_timeout(timeout)
            .then(|| ())
            .ok_or(anyhow!("Waiting for cancellation timeout"))
    }

    /// Cancel the currently running task instance and async-await it.
    pub async fn cancel_and_async_wait(&self) -> AnyResult<()> {
        self.cancel()?;

        self.event.listen().await;
        Ok(())
    }

    fn cancel(&self) -> AnyResult<()> {
        unsafe {
            GLOBAL_TIMER_EVENT_SENDER
                .as_ref()
                .map(|s| {
                    s.try_send(TimerEvent::CancelTask(self.task_id, self.record_id))
                        .with_context(|| "Failed Send Event from seed_timer_event".to_string())
                })
                .ok_or(anyhow!("GLOBAL_TIMER_EVENT_SENDER isn't init."))?
        }
    }
}

impl TaskInstancesChain {}

impl Default for TaskInstancesChain {
    fn default() -> Self {
        let shared_list: Arc<LinkedList<Arc<Instance>>> = Arc::new(LinkedList::new());
        let inner: Arc<ArcSwap<LinkedList<Arc<Instance>>>> = Arc::new(ArcSwap::new(shared_list));

        TaskInstancesChain { inner }
    }
}

impl From<&TaskInstancesChain> for TaskInstancesChainMaintainer {
    fn from(value: &TaskInstancesChain) -> TaskInstancesChainMaintainer {
        let inner = Arc::downgrade(&value.inner);
        TaskInstancesChainMaintainer { inner }
    }
}

impl TaskInstancesChainMaintainer {}
