use super::task::Task;
use std::collections::LinkedList;

//Slot 基于 链表做一个有序，如果头部执行了，直接插入其它SLOT
//如果，新增 则根据圈数 找链表可以插入的节点插入就可以了
//支持一次遍历，全部的圈数-1
pub struct Slot {
    tasks: LinkedList<u32>,
}

impl Slot {
    pub(crate) fn new() -> Self {
        Slot {
            tasks: LinkedList::new(),
        }
    }

    pub(crate) fn add_task(&mut self, task: u32) {
        if self.tasks.len() == 0 {
            self.tasks.push_front(task);
            return;
        }

        let mut split_num = 0;

        // 在指定链表节点后面插入
        //https://doc.rust-lang.org/std/collections/linked_list/struct.CursorMut.html
        // for task_of_tasks in self.tasks.iter() {
        //     if task_of_tasks.cylinder_line > task.cylinder_line {
        //         break;
        //     }
        //     split_num += 1;
        // }

        // let tmp_slipt_tasks = self.tasks.split_off_after_node(split_node: Option<NonNull<Node<T>>>, at: usize)
    }

    //这一时刻该执行的任务
    //cylinder_line == 0
    pub fn arrival_time_tasks() {

        //重task 中 clone 闭包，发送到外面
        //内部重新计算圈数
        //发送给Timer 让他去决定插到哪里
    }
}
