use crate::prelude::*;

use std::collections::LinkedList;
use std::convert::From;
use std::sync::{Arc, Weak};
use std::time::Duration;

use anyhow::{anyhow, Context, Result as AnyResult};
use event_listener::Event;
use futures::executor::block_on;

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
pub(crate) type InstanceList = Arc<LinkedList<Arc<Instance>>>;

/// Chain of task run instances.
/// For User access to Running-Task's instance.
#[derive(Debug)]
pub struct TaskInstancesChain {
    pub(crate) inner: Arc<AsyncRwLock<InstanceList>>,
}

/// Chain of task run instances.
/// For inner maintain to Running-Task's instance.
#[derive(Debug, Default)]
pub struct TaskInstancesChainMaintainer {
    pub(crate) inner: Weak<AsyncRwLock<InstanceList>>,
}

impl Instance {
    pub(crate) fn get_task_id(&self) -> u64 {
        self.task_id
    }

    pub(crate) fn get_record_id(&self) -> i64 {
        self.record_id
    }

    pub(crate) fn set_task_id(mut self, task_id: u64) -> Instance {
        self.task_id = task_id;
        self
    }

    pub(crate) fn set_record_id(mut self, record_id: i64) -> Instance {
        self.record_id = record_id;
        self
    }

    pub(crate) fn notify_cancel_finish(&self) {
        self.event.notify(usize::MAX);
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

impl TaskInstancesChain {
    // sync context.
    pub fn get_instance_list(&self) -> InstanceList {
        // Just clone Arc don't keeping lock.
        block_on(self.inner.read()).clone()
    }

    // async context
    pub async fn get_instance_list_and_async_await(&self) -> InstanceList {
        // Just clone Arc don't keeping lock.
        self.inner.read().await.clone()
    }
}

impl Default for TaskInstancesChain {
    fn default() -> Self {
        let shared_list: InstanceList = Arc::new(LinkedList::new());
        let inner: Arc<AsyncRwLock<InstanceList>> = Arc::new(AsyncRwLock::new(shared_list));

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
