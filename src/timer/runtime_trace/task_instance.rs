use crate::prelude::*;

use std::collections::linked_list::Iter;
use std::collections::LinkedList;
use std::iter::Peekable;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result as AnyResult};
use event_listener::Event;
use smol::channel::{unbounded, Receiver, Sender};

/// instance of task running.
#[derive(Debug, Default)]
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

    pub(crate) fn set_header_state(&self, state: usize) {
        self.header.state.store(state, Ordering::Release);
    }

    pub(crate) fn notify_cancel_finish(&self) {
        self.set_header_state(state::instance::CANCELLED);
        self.header.event.notify(usize::MAX);
    }

    pub(crate) fn notify_complete_finish(&self) {
        self.set_header_state(state::instance::COMPLETED);
        self.header.event.notify(usize::MAX);
    }

    /// Cancel the currently running task instance and block the thread to wait.
    pub fn cancel_with_wait(&self) -> AnyResult<()> {
        self.cancel()?;

        self.header.event.listen().wait();
        Ok(())
    }

    /// Cancel the currently running task instance and block the thread to wait
    /// for an expected amount of time.
    pub fn cancel_with_wait_timeout(&self, timeout: Duration) -> AnyResult<()> {
        self.cancel()?;

        self.header
            .event
            .listen()
            .wait_timeout(timeout)
            .then(|| ())
            .ok_or(anyhow!("Waiting for cancellation timeout"))
    }

    /// Cancel the currently running task instance and async-await it.
    pub async fn cancel_with_async_wait(&self) -> AnyResult<()> {
        self.cancel()?;

        self.header.event.listen().await;
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
