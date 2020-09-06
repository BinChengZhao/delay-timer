use super::task::Task;
use std::collections::HashMap;

//Slot is based on HashMap, It easy to add it and find it.
pub struct Slot {
    task_map: HashMap<usize, Task>,
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

    pub(crate) fn remove_task(&mut self, task_id: usize) -> Option<Task> {
        self.task_map.remove(&task_id)
    }

    //check并减cylinder_line，
    //返回现在要运行的，TaskOwned的集合
    //cylinder_line == 0
    pub fn arrival_time_tasks(&mut self) -> Vec<usize> {
        let mut task_id_vec = vec![];

        for (_, task) in self.task_map.iter_mut() {
            if task.check_arrived() {
                task_id_vec.push(task.task_id);
            }
        }

        task_id_vec
    }
}
