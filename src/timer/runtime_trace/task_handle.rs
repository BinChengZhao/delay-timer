//TaskTrace-全局的句柄
//当进程消亡，跟异步任务drop的时候对应的链表也减少，如果没值则删除k/v
//如果是单实例执行任务，查看对应id是否有句柄在链表，如果有则跳过
//如果是可多实例执行，直接追加新句柄在链表后
//每个任务执行时，挂一个async的计时器，到时间时去调用句柄的cancel，调度者不负责取消超时

use anyhow::Result;
use smol::Task as SmolTask;
use std::collections::{HashMap, LinkedList};
use std::process::Child;

#[derive(Default)]
pub(crate) struct TaskTrace {
    inner: HashMap<u32, LinkedList<DelayTaskHandlerBox>>,
}

impl TaskTrace {
    pub(crate) fn new() -> Self {
        TaskTrace {
            inner: HashMap::with_capacity(100),
        }
    }

    pub(crate) fn insert(&mut self, task_id: u32, task_handler_box: DelayTaskHandlerBox) {
        //entry is amazing!
        self.inner
            .entry(task_id)
            .or_insert(LinkedList::new())
            .push_back(task_handler_box);
    }

    pub(crate) fn clear(self) {
        for (_task_id, task_handler_box_list) in self.inner.into_iter() {
            for task_handler_box in task_handler_box_list.into_iter() {
                drop(task_handler_box);
            }
        }
    }
}

//I export that trait for that crate user.
pub trait DelayTaskHandler {
    fn stop(self:Box<Self>) -> Result<()>;
}

// The problem of storage diffrent type in  DelayTaskHandlerBox  was solved through dyn DelayTaskHandler.
// Before thinking about this solution, I thought about enumerations and generics Type.
// But Both of them with new problems will be introduced , enumerations can't allow crate user expand,
// generics Type will single state just store one type in TaskTrace.
// Multi-DelayTaskHandlerBox record_id can same, because one task can spawn Multi-process.
pub(crate) struct DelayTaskHandlerBox {
    task_handler: Option<Box<dyn DelayTaskHandler>>,
    task_id: u32,
    record_id: u64,
    start_time: u32,
}

impl Drop for DelayTaskHandlerBox {
    fn drop(&mut self) {
        if let Some(mut task_handler) = self.task_handler.take(){
            //使用trait 对象，不能直接传所有权，因为大小不确定
            //所以我用Box包装一下
            task_handler.stop();
        }
    }
}

#[derive(Default)]
pub(crate) struct DelayTaskHandlerBoxBuilder {
    task_id: u32,
    record_id: u64,
    start_time: u32,
}

impl DelayTaskHandlerBoxBuilder {
    pub fn set_task_id(&mut self, task_id: u32) {
        self.task_id = task_id;
    }
    pub fn set_record_id(&mut self, record_id: u64) {
        self.record_id = record_id;
    }
    pub fn set_start_time(&mut self, start_time: u32) {
        self.start_time = start_time;
    }

    pub fn spawn(self, task_handler: Box<DelayTaskHandler>) -> DelayTaskHandlerBox {
        DelayTaskHandlerBox {
            task_handler: Some(task_handler),
            task_id: self.task_id,
            record_id: self.record_id,
            start_time: self.start_time,
        }
    }
}

impl DelayTaskHandlerBox {
    pub fn get_task_id(&mut self) -> u32 {
        self.task_id
    }
    pub fn get_record_id(&mut self) -> u64 {
        self.record_id
    }
    pub fn get_start_time(&mut self) -> u32 {
        self.start_time
    }
}

//Deafult implementation for Child and SmolTask
//TODO:Maybe i can implementation a proc macro.

impl DelayTaskHandler for Child {
    fn stop(mut self : Box<Self>) -> Result<()> {
        //to anyhow:Result
        self.kill()?;
        Ok(())
    }
}

//When SmolTask is dropped, async task is cancel.
impl DelayTaskHandler for SmolTask<Result<()>> {
    fn stop(self: Box<Self>) -> Result<()> {
        drop(self);
        println!("bye bye  i'm  async");
        Ok(())
    }
}
