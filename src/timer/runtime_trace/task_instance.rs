use crate::prelude::*;

use std::convert::From;
use std::time::Duration;
use std::sync::{Arc, Weak};
use std::collections::LinkedList;

use arc_swap::ArcSwap;
use event_listener::Event;
use anyhow::{Result as AnyResult};

/// instance of task running.
#[derive(Debug)]
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
    pub(crate) inner: Arc<ArcSwap<LinkedList<Weak<Instance>>>>,
}

/// Chain of task run instances.
/// For inner maintain to Running-Task's instance.
#[derive(Debug, Clone)]
pub(crate) struct TaskInstancesChainMaintainer {
    pub(crate) inner: Weak<ArcSwap<LinkedList<Weak<Instance>>>>,
}

impl Instance{
    /// Cancel the currently running task instance and block the thread to wait.
    pub fn cancel_and_wait(&self) ->AnyResult<()> {
        self.event.listen().wait();
        todo!()
    }

    /// Cancel the currently running task instance and block the thread to wait
    /// for an expected amount of time.
    pub fn cancel_and_wait_timeout(&self, timeout:Duration) ->AnyResult<()> {
        self.event.listen().wait_timeout(timeout);
        todo!()
    }

    /// Cancel the currently running task instance and async-await it.
    pub async fn cancel_and_async_wait(&self) ->AnyResult<()> {
        self.event.listen().await;
        todo!()
    }
}

impl TaskInstancesChain {}

impl Default for TaskInstancesChain {
    fn default() -> Self {
        let shared_list: Arc<LinkedList<Weak<Instance>>> = Arc::new(LinkedList::new());
        let inner: Arc<ArcSwap<LinkedList<Weak<Instance>>>> = Arc::new(ArcSwap::new(shared_list));

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
