///TaskTrace own global task-handle.
//当进程消亡，跟异步任务drop的时候对应的链表也减少，如果没值则删除k/v
//如果是单实例执行任务，查看对应id是否有句柄在链表，如果有则跳过
//如果是可多实例执行，直接追加新句柄在链表后
use anyhow::Result;
use smol::Task as SmolTask;
use std::collections::{HashMap, LinkedList};
use std::process::Child;

#[derive(Default)]
// TaskTrace is contanier
pub(crate) struct TaskTrace {
    inner: HashMap<u64, LinkedList<DelayTaskHandlerBox>>,
}

//hashMqp  task_id => child-handle-linklist
//可以取消任务，child-handle 可以是进程句柄 - 也可以是异步句柄， 用linklist 是因为，可能任务支持同时多个并行
impl TaskTrace {
    pub(crate) fn insert(&mut self, task_id: u64, task_handler_box: DelayTaskHandlerBox) {
        //entry is amazing!
        self.inner
            .entry(task_id)
            .or_insert_with(LinkedList::new)
            .push_back(task_handler_box);
    }

    pub(crate) fn clear(self) {
        for (_task_id, task_handler_box_list) in self.inner.into_iter() {
            for task_handler_box in task_handler_box_list.into_iter() {
                drop(task_handler_box);
            }
        }
    }

    //linkedlist is ordered by record_id, if input record_id is small than linkedlist first record_id
    //that is no task_handler can cancel  or record_id bigger than last record_id.
    //one record_id may be used for many handler.
    pub(crate) fn quit_one_task_handler(
        &mut self,
        task_id: u64,
        record_id: i64,
    ) -> Option<Result<()>> {
        let task_handler_list = self.inner.get_mut(&task_id)?;

        //TODO: Optimize.
        let filter_collection =
            task_handler_list.drain_filter(|handler_box| handler_box.record_id == record_id);

        let (filter_collection_count, _) = filter_collection.size_hint();

        if filter_collection_count == 0 {
            return None;
        }

        let mut handlers_quit_result = Some(Ok(()));

        for mut task_handler_box in filter_collection {
            let handler_quit_result = task_handler_box.quit();
            if handler_quit_result.is_err() {
                handlers_quit_result = Some(handler_quit_result);
            }
        }

        handlers_quit_result
    }
}

//I export that trait for that crate user.
pub trait DelayTaskHandler: Send + Sync {
    fn quit(self: Box<Self>) -> Result<()>;
}

// The problem of storage diffrent type in  DelayTaskHandlerBox  was solved through dyn DelayTaskHandler.
// Before thinking about this solution, I thought about enumerations and generics Type.
// But Both of them with new problems will be introduced , enumerations can't allow crate user expand,
// generics Type will single state just store one type in TaskTrace.
// Multi-DelayTaskHandlerBox record_id can same, because one task can spawn Multi-process.
pub(crate) struct DelayTaskHandlerBox {
    task_handler: Option<Box<dyn DelayTaskHandler>>,
    task_id: u64,
    ///Globally unique ID.
    record_id: i64,
    start_time: u64,
    end_time: Option<u64>,
}

impl Drop for DelayTaskHandlerBox {
    fn drop(&mut self) {
        if let Some(task_handler) = self.task_handler.take() {
            //使用trait 对象，不能直接传所有权，因为大小不确定
            //所以我用Box包装一下
            task_handler.quit();
        }
    }
}

#[derive(Default)]
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
    #[inline(always)]
    pub fn get_task_id(&self) -> u64 {
        self.task_id
    }

    #[inline(always)]
    pub fn get_record_id(&self) -> i64 {
        self.record_id
    }

    #[inline(always)]
    pub fn get_start_time(&self) -> u64 {
        self.start_time
    }

    #[inline(always)]
    pub fn get_end_time(&self) -> Option<u64> {
        self.end_time
    }

    fn quit(&mut self) -> Result<()> {
        if let Some(task_handler) = self.task_handler.take() {
            return task_handler.quit();
        }

        Ok(())
    }
}

//Deafult implementation for Child and SmolTask
//TODO:Maybe i can implementation a proc macro.

impl DelayTaskHandler for Child {
    fn quit(mut self: Box<Self>) -> Result<()> {
        //to anyhow:Result
        self.kill()?;
        Ok(())
    }
}

impl DelayTaskHandler for LinkedList<Child> {
    fn quit(mut self: Box<Self>) -> Result<()> {
        //to anyhow:Result

        for child in (*self).iter_mut() {
            //TODO:Maybe first child kill fail after childs will be leak.
            child.kill()?;
        }
        Ok(())
    }
}

//When SmolTask is dropped, async task is cancel.
impl DelayTaskHandler for SmolTask<Result<()>> {
    fn quit(self: Box<Self>) -> Result<()> {
        smol::spawn(async {
            self.cancel().await;
        })
        .detach();
        println!("bye bye  i'm  async");
        Ok(())
    }
}
