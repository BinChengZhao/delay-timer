use crate::prelude::*;

use std::collections::LinkedList;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

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

/// Public instance of task running.
#[derive(Debug, Clone)]
pub struct TaskInstance {
    pub(crate) instance: Instance,
    pub(crate) timer_event_sender: TimerEventSender,
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
        timer_event_sender: None,
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
    pub(crate) timer_event_sender: Option<TimerEventSender>,
}

/// Chain of task run instances.
/// For inner maintain to Running-Task's instance.
#[derive(Debug)]
pub struct TaskInstancesChainMaintainer {
    pub(crate) inner_sender: Sender<Instance>,
    pub(crate) inner_state: Arc<AtomicUsize>,
    pub(crate) inner_list: LinkedList<Instance>,
}

impl Instance {
    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn get_task_id(&self) -> u64 {
        self.task_id
    }

    #[inline(always)]
    pub(crate) fn get_record_id(&self) -> i64 {
        self.record_id
    }

    #[inline(always)]
    pub(crate) fn set_task_id(mut self, task_id: u64) -> Instance {
        self.task_id = task_id;
        self
    }

    #[inline(always)]
    pub(crate) fn set_record_id(mut self, record_id: i64) -> Instance {
        self.record_id = record_id;
        self
    }

    #[inline(always)]
    pub(crate) fn set_state(&self, state: usize) {
        self.header.state.store(state, Ordering::Release);
    }

    /// Get state of Instance.
    #[inline(always)]
    pub fn get_state(&self) -> InstanceState {
        self.header.state.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub(crate) fn notify_cancel_finish(&self, state: usize) {
        self.set_state(state);
        self.header.event.notify(usize::MAX);
    }
}

impl TaskInstance {
    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn get_task_id(&self) -> u64 {
        self.instance.get_task_id()
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn get_record_id(&self) -> i64 {
        self.instance.get_record_id()
    }

    /// Get state of Instance.
    #[inline(always)]
    pub fn get_state(&self) -> InstanceState {
        self.instance.get_state()
    }

    /// Cancel the currently running task instance and block the thread to wait.
    #[inline(always)]
    pub fn cancel_with_wait(&self) -> Result<InstanceState, TaskInstanceError> {
        self.cancel()?;

        self.instance.header.event.listen().wait();
        Ok(self.get_state())
    }

    /// Cancel the currently running task instance and block the thread to wait
    /// for an expected amount of time.
    #[inline(always)]
    pub fn cancel_with_wait_timeout(
        &self,
        timeout: Duration,
    ) -> Result<InstanceState, TaskInstanceError> {
        self.cancel()?;

        self.instance
            .header
            .event
            .listen()
            .wait_timeout(timeout)
            .then(|| self.get_state())
            .ok_or(TaskInstanceError::DisCancelTimeOut)
    }

    /// Cancel the currently running task instance and async-await it.
    #[inline(always)]
    pub async fn cancel_with_async_wait(&self) -> Result<InstanceState, TaskInstanceError> {
        self.cancel()?;

        self.instance.header.event.listen().await;
        Ok(self.get_state())
    }

    #[inline(always)]
    fn cancel(&self) -> Result<(), TaskInstanceError> {
        if self.get_state() != state::instance::RUNNING {
            return Err(TaskInstanceError::DisCancel);
        }

        Ok(self.timer_event_sender.try_send(TimerEvent::CancelTask(
            self.instance.task_id,
            self.instance.record_id,
        ))?)
    }
}

impl TaskInstancesChainMaintainer {
    pub(crate) async fn push_instance(&mut self, instance: Instance) {
        self.inner_sender
            .send(instance.clone())
            .await
            .map_err(|e| {
                tracing::error!("push_instance error: {:?}", e);
            })
            .ok();
        self.inner_list.push_back(instance);
    }
}
impl TaskInstancesChain {
    /// Non-blocking get the next task instance.
    pub fn next(&self) -> Result<TaskInstance, TaskInstanceError> {
        let timer_event_sender = self.get_timer_event_sender()?;

        Ok(self
            .inner_receiver
            .try_recv()
            .map(|instance| TaskInstance {
                instance,
                timer_event_sender,
            })?)
    }

    /// Blocking get the next task instance.
    pub fn next_with_wait(&self) -> Result<TaskInstance, TaskInstanceError> {
        let timer_event_sender = self.get_timer_event_sender()?;

        let instance = block_on(self.inner_receiver.recv())?;

        Ok(TaskInstance {
            instance,
            timer_event_sender,
        })
    }

    /// Async-await get the next task instance.
    pub async fn next_with_async_wait(&self) -> Result<TaskInstance, TaskInstanceError> {
        let timer_event_sender = self.get_timer_event_sender()?;

        let instance = self.inner_receiver.recv().await?;

        Ok(TaskInstance {
            instance,
            timer_event_sender,
        })
    }

    /// Get state of TaskInstancesChain.
    #[inline(always)]
    fn get_state(&self) -> InstanceState {
        self.inner_state.load(Ordering::Acquire)
    }

    fn get_timer_event_sender(&self) -> Result<Sender<TimerEvent>, TaskInstanceError> {
        if self.get_state() == state::instance_chain::ABANDONED {
            return Err(TaskInstanceError::Expired);
        }

        self.timer_event_sender
            .clone()
            .ok_or(TaskInstanceError::MisEventSender)
    }
}

impl Drop for TaskInstancesChain {
    fn drop(&mut self) {
        self.inner_state
            .store(state::instance_chain::DROPPED, Ordering::Release);
    }
}

impl Drop for TaskInstancesChainMaintainer {
    fn drop(&mut self) {
        self.inner_state
            .store(state::instance_chain::ABANDONED, Ordering::Release);
    }
}
