//! Internal-task-handle
//! The internal-task-handle, which holds the execution handle of the running task,
//! gives lib the support to exit the task at any time.
use crate::prelude::*;

use std::collections::{HashMap, LinkedList};
use std::fmt::{self, Debug, Formatter, Pointer};

use anyhow::Result;
use smol::Task as SmolTask;

#[derive(Default, Debug)]
/// TaskTrace is contanier that own global task-handle.
pub(crate) struct TaskTrace {
    inner: HashMap<u64, LinkedList<DelayTaskHandlerBox>>,
}

// TaskTrace can cancel a task via a Task Handle.
impl TaskTrace {
    pub(crate) fn insert(&mut self, task_id: u64, task_handler_box: DelayTaskHandlerBox) {
        //entry is amazing!
        self.inner
            .entry(task_id)
            .or_insert_with(LinkedList::new)
            .push_back(task_handler_box);
    }

    #[allow(dead_code)]
    pub(crate) fn clear(self) {
        for (_task_id, task_handler_box_list) in self.inner.into_iter() {
            for task_handler_box in task_handler_box_list.into_iter() {
                drop(task_handler_box);
            }
        }
    }

    // linkedlist is ordered by record_id, if input record_id is small than linkedlist first record_id
    // that is no task_handler can cancel  or record_id bigger than last record_id.
    // one record_id may be used for many handler.

    //TODO: One stable cfg-flag， One nightly cfg-flag .

    #[cfg(RUSTC_IS_NIGHTLY)]
    pub(crate) fn quit_one_task_handler(&mut self, task_id: u64, record_id: i64) -> Result<()> {
        let task_handler_list = self.inner.get_mut(&task_id).ok_or_else(|| {
            anyhow!(
                "Fn : `quit_one_task_handler`, No task-handler-list found (task-id: {}, record-id: {} )",
                task_id, record_id
            )
        })?;

        let mut list_mut_cursor = task_handler_list.cursor_front_mut();

        let mut task_handler_box_ref: &mut DelayTaskHandlerBox;
        loop {
            task_handler_box_ref = list_mut_cursor.current().ok_or_else(|| {
                anyhow!(
                    "Fn : `quit_one_task_handler`, No task_handler_box_ref found (task-id: {}, record-id: {} )",
                    task_id, record_id
                )
            })?;

            if task_handler_box_ref.record_id > record_id {
                return Err(anyhow!(
                    "Fn : `quit_one_task_handler`, No task_handler found (task-id: {}, record-id: {} )",
                    task_id, record_id
                ));
            }

            if task_handler_box_ref.record_id == record_id {
                break;
            }

            list_mut_cursor.move_next();
        }

        // remove current task_handler_box.
        list_mut_cursor
            .remove_current()
            .map(|mut task_handler_box| task_handler_box.quit())
            .transpose()?;

        Ok(())
    }

    #[cfg(not(RUSTC_IS_NIGHTLY))]
    pub(crate) fn quit_one_task_handler(&mut self, task_id: u64, record_id: i64) -> Result<()> {
        let task_handler_list = self.inner.get_mut(&task_id).ok_or_else(|| {
            anyhow!(
                "Fn : `quit_one_task_handler`, No task-handler-list found (task-id: {} )",
                task_id
            )
        })?;
        let index = task_handler_list
            .iter()
            .position(|d| d.record_id == record_id)
            .ok_or_else(|| {
                anyhow!(
                    "Fn : `quit_one_task_handler`, No task-handle-index found (task-id: {} , record-id: {})",
                    task_id,
                    record_id
                )
            })?;

        let mut has_remove_element_list = task_handler_list.split_off(index);

        let mut remove_element = has_remove_element_list.pop_front().ok_or_else(|| {
            anyhow!(
                "No task-handle found in list (task-id: {} , record-id: {})",
                task_id,
                record_id
            )
        })?;

        task_handler_list.append(&mut has_remove_element_list);

        remove_element.quit()
    }
}

//I export that trait for that crate user.
/// You can implement this trait for your type `T`,
/// and then you can define the function that returns the `Box<T> as Box<dyn DelayTaskHandler>` closure,
/// which can be wrapped by the TaskBuilder and then thrown into the time wheel for constant rotation.
pub trait DelayTaskHandler: Send + Sync {
    /// Stopping a running task instance.
    fn quit(self: Box<Self>) -> Result<()>;
}

pub struct SafeStructBoxedDelayTaskHandler(pub(crate) Box<dyn DelayTaskHandler>);
impl Debug for SafeStructBoxedDelayTaskHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <&Self as Pointer>::fmt(&self, f)
    }
}

impl SafeStructBoxedDelayTaskHandler {
    pub(crate) fn get_inner(self) -> Box<dyn DelayTaskHandler> {
        self.0
    }
}

// The problem of storage diffrent type in  DelayTaskHandlerBox  was solved through dyn DelayTaskHandler.
// Before thinking about this solution, I thought about enumerations and generics Type.
// But Both of them with new problems will be introduced , enumerations can't allow crate user expand,
// generics Type will single state just store one type in TaskTrace.
// Multi-DelayTaskHandlerBox record_id can same, because one task can spawn Multi-process.
#[derive(Debug)]
pub struct DelayTaskHandlerBox {
    ///Task Handle is most important part of DelayTaskHandlerBox.
    pub(crate) task_handler: Option<SafeStructBoxedDelayTaskHandler>,
    ///task_id.
    task_id: u64,
    ///Globally unique ID.
    record_id: i64,
    ///it's start_time.
    #[allow(dead_code)]
    start_time: u64,
    ///it's end_time.
    end_time: Option<u64>,
}

impl Drop for DelayTaskHandlerBox {
    fn drop(&mut self) {
        if let Some(task_handler) = self.task_handler.take() {
            //Using a trait object, you can't pass ownership directly because the size is uncertain.
            //So packet it by `Box`.
            task_handler
                .get_inner()
                .quit()
                .unwrap_or_else(|e| error!(" `DelayTaskHandlerBox::drop` : {}", e));
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct DelayTaskHandlerBoxBuilder {
    task_id: u64,
    record_id: i64,
    start_time: u64,
    end_time: Option<u64>,
}

impl DelayTaskHandlerBoxBuilder {
    #[inline(always)]
    pub fn set_task_id(mut self, task_id: u64) -> Self {
        self.task_id = task_id;
        self
    }

    #[inline(always)]
    pub fn set_record_id(mut self, record_id: i64) -> Self {
        self.record_id = record_id;
        self
    }

    #[inline(always)]
    pub fn set_start_time(mut self, start_time: u64) -> Self {
        self.start_time = start_time;
        self
    }

    pub fn set_end_time(mut self, maximum_running_time: Option<u64>) -> Self {
        self.end_time = maximum_running_time;

        self
    }

    pub fn spawn(self, task_handler: Box<dyn DelayTaskHandler>) -> DelayTaskHandlerBox {
        let task_handler = SafeStructBoxedDelayTaskHandler(task_handler);
        DelayTaskHandlerBox {
            task_handler: Some(task_handler),
            task_id: self.task_id,
            record_id: self.record_id,
            start_time: self.start_time,
            end_time: self.end_time,
        }
    }
}

impl DelayTaskHandlerBox {
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_task_handler(&self) -> &Option<SafeStructBoxedDelayTaskHandler> {
        &(self.task_handler)
    }

    #[inline(always)]
    pub fn get_task_id(&self) -> u64 {
        self.task_id
    }

    #[inline(always)]
    pub fn get_record_id(&self) -> i64 {
        self.record_id
    }

    #[inline(always)]
    pub fn get_end_time(&self) -> Option<u64> {
        self.end_time
    }

    fn quit(&mut self) -> Result<()> {
        if let Some(task_handler) = self.task_handler.take() {
            return task_handler.get_inner().quit();
        }

        Ok(())
    }
}

//Deafult implementation for Child and SmolTask
//TODO:Maybe i can implementation a proc macro.

impl<Child: ChildUnify> DelayTaskHandler for ChildGuard<Child> {
    fn quit(self: Box<Self>) -> Result<()> {
        drop(self);
        Ok(())
    }
}

impl<Child: ChildUnify> DelayTaskHandler for ChildGuardList<Child> {
    fn quit(self: Box<Self>) -> Result<()> {
        drop(self);
        Ok(())
    }
}

impl DelayTaskHandler for () {
    fn quit(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

//When SmolTask is dropped, async task is cancel.
impl<T: Send + Sync + 'static> DelayTaskHandler for SmolTask<T> {
    fn quit(self: Box<Self>) -> Result<()> {
        smol::spawn(async {
            self.cancel().await;
        })
        .detach();
        Ok(())
    }
}

use tokio::task::JoinHandle;
impl<T: Send + Sync + Debug + 'static> DelayTaskHandler for JoinHandle<T> {
    fn quit(self: Box<Self>) -> Result<()> {
        (*self).abort();
        Ok(())
    }
}
