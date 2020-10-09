use super::task::Task;
use std::collections::HashMap;

//Slot is based on HashMap, It easy to add it and find it.
pub struct Slot {
    //The scale of the clock, the task source is maintained by a hash table,
    //the addition and removal of tasks is O(1),
    //and the subtraction of laps is O(n).
    task_map: HashMap<u64, Task>,
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

    pub(crate) fn remove_task(&mut self, task_id: u64) -> Option<Task> {
        self.task_map.remove(&task_id)
    }

    //check并减cylinder_line，
    //返回现在要运行的，TaskOwned的集合
    //cylinder_line == 0
    pub(crate) fn arrival_time_tasks(&mut self) -> Vec<u64> {
        let mut task_id_vec = vec![];

        for (_, task) in self.task_map.iter_mut() {
            if task.check_arrived() {
                task_id_vec.push(task.task_id);
            }
        }

        task_id_vec
    }
}
