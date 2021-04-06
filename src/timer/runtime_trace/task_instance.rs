use crate::prelude::*;

use std::collections::LinkedList;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result as AnyResult};
use event_listener::Event;
use future_lite::block_on;
use smol::channel::{unbounded, Receiver, Sender};

/// instance of task running.
#[derive(Debug, Default, Clone)]
pub struct Instance {
    /// The header of the taskInstance.
    header: Arc<InstanceHeader>,
    /// The id of task.
    task_id: u64,
    /// The id of task running record.
    record_id: i64,
}

#[derive(Debug)]
pub(crate) struct InstanceHeader {
    /// The event view of inner taskInstance.
    event: Event,
    /// The state of inner taskInstance.
    state: AtomicUsize,
}

impl Default for InstanceHeader {
    fn default() -> Self {
        let event = Event::new();
        let state = AtomicUsize::new(state::instance::RUNNING);

        InstanceHeader { event, state }
    }
}

pub(crate) fn task_instance_chain_pair() -> (TaskInstancesChain, TaskInstancesChainMaintainer) {
    let (inner_sender, inner_receiver) = unbounded::<Instance>();
    let inner_state = Arc::new(AtomicUsize::new(state::instance_chain::LIVING));
    let inner_list = LinkedList::new();

    let chain = TaskInstancesChain {
        inner_receiver,
        inner_state: inner_state.clone(),
    };
    let chain_maintainer = TaskInstancesChainMaintainer {
        inner_sender,
        inner_state,
        inner_list,
    };

    (chain, chain_maintainer)
}

/// Chain of task run instances.
/// For User access to Running-Task's instance.
#[derive(Debug)]
pub struct TaskInstancesChain {
    pub(crate) inner_receiver: Receiver<Instance>,
    pub(crate) inner_state: Arc<AtomicUsize>,
}

/// Chain of task run instances.
/// For inner maintain to Running-Task's instance.
#[derive(Debug)]
pub struct TaskInstancesChainMaintainer {
    //TODO: Maybe needed be Arc<AsyncRwLock<InstanceListInner>>.
    pub(crate) inner_sender: Sender<Instance>,
    pub(crate) inner_state: Arc<AtomicUsize>,
    pub(crate) inner_list: LinkedList<Instance>,
}

impl Instance {
    #[allow(dead_code)]
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

    pub(crate) fn set_state(&self, state: usize) {
        self.header.state.store(state, Ordering::Release);
    }

    /// Get state of Instance.
    pub fn get_state(&self) -> InstanceState {
        self.header.state.load(Ordering::Acquire)
    }
    pub(crate) fn notify_cancel_finish(&self, state: usize) {
        self.set_state(state);
        self.header.event.notify(usize::MAX);
    }

    /// Cancel the currently running task instance and block the thread to wait.
    pub fn cancel_with_wait(&self) -> AnyResult<InstanceState> {
        self.cancel()?;

        self.header.event.listen().wait();
        Ok(self.get_state())
    }

    /// Cancel the currently running task instance and block the thread to wait
    /// for an expected amount of time.
    pub fn cancel_with_wait_timeout(&self, timeout: Duration) -> AnyResult<InstanceState> {
        self.cancel()?;

        self.header
            .event
            .listen()
            .wait_timeout(timeout)
            .then(|| self.get_state())
            .ok_or_else(|| anyhow!("Waiting for cancellation timeout"))
    }

    /// Cancel the currently running task instance and async-await it.
    pub async fn cancel_with_async_wait(&self) -> AnyResult<InstanceState> {
        self.cancel()?;

        self.header.event.listen().await;
        Ok(self.get_state())
    }

    fn cancel(&self) -> AnyResult<()> {
        unsafe {
            GLOBAL_TIMER_EVENT_SENDER
                .as_ref()
                .map(|s| {
                    s.try_send(TimerEvent::CancelTask(self.task_id, self.record_id))
                        .with_context(|| "Failed Send Event from seed_timer_event".to_string())
                })
                .ok_or_else(|| anyhow!("GLOBAL_TIMER_EVENT_SENDER isn't init."))?
        }
    }
}

impl TaskInstancesChainMaintainer {
    pub(crate) async fn push_instance(&mut self, instance: Instance) {
        self.inner_sender
            .send(instance.clone())
            .await
            .map_or((), |_| {});
        self.inner_list.push_back(instance);
    }
}
impl TaskInstancesChain {
    /// Non-blocking get the next task instance.
    pub fn next(&self) -> AnyResult<Instance> {
        Ok(self.inner_receiver.try_recv()?)
    }

    /// Blocking get the next task instance.
    pub fn next_with_wait(&self) -> AnyResult<Instance> {
        Ok(block_on(self.inner_receiver.recv())?)
    }

    /// Async-await get the next task instance.
    pub async fn next_with_async_wait(&self) -> AnyResult<Instance> {
        Ok(self.inner_receiver.recv().await?)
    }
}

impl Drop for TaskInstancesChain {
    fn drop(&mut self) {
        self.inner_state
            .store(state::instance_chain::DROPPED, Ordering::Release);
    }
}
