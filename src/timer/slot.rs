use super::task::Task;
use std::collections::HashMap;

//Slot 基于 链表做一个有序，如果头部执行了，直接插入其它SLOT
//如果，新增 则根据圈数 找链表可以插入的节点插入就可以了
//支持一次遍历，全部的圈数-1
pub struct Slot {
    task_map: HashMap<u32, Task>,
}

impl Slot {
    pub(crate) fn new() -> Self {
        Slot {
            task_map: HashMap::new(),
        }
    }

    pub(crate) fn add_task(&mut self, task: Task) -> Option<Task> {
        self.task_map.insert(task.task_id, task)
    }

    pub(crate) fn remove_task(&mut self, task_id: u32) -> Option<Task> {
        self.task_map.remove(&task_id)
    }

    //check并减cylinder_line，
    //返回现在要运行的，TaskOwned的集合
    //cylinder_line == 0
    pub fn arrival_time_tasks() {

        //重task 中 clone 闭包，发送到外面
        //内部重新计算圈数
        //发送给Timer 让他去决定插到哪里
    }
}
