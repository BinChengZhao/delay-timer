//! Slot
//! It is the scale of the internal clock.
use super::task::Task;
use std::collections::HashMap;
use std::mem::swap;

//Slot is based on HashMap, It easy to add it and find it.
pub(crate) struct Slot {
    // The scale of the clock, the task source is maintained by a hash table,
    // the addition and removal of tasks is O(1),
    // and the subtraction of laps is O(n).
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

    pub(crate) fn update_task(&mut self, mut task: Task) -> Option<Task> {
        match self.task_map.get_mut(&task.task_id) {
            Some(t) => {
                swap(t, &mut task);
                Some(task)
            }

            None => self.task_map.insert(task.task_id, task),
        }
    }

    pub(crate) fn remove_task(&mut self, task_id: u64) -> Option<Task> {
        self.task_map.remove(&task_id)
    }

    // Check and reduce cylinder_line，
    // Returns a Vec. containing all task ids to be executed.(cylinder_line == 0)
    pub(crate) fn arrival_time_tasks(&mut self) -> Vec<u64> {
        let mut task_id_vec = vec![];

        for (_, task) in self.task_map.iter_mut() {
            if task.check_arrived() {
                task_id_vec.push(task.task_id);
            }
        }

        task_id_vec
    }

    // When the operation is finished with the task, shrink the container in time
    // To avoid the overall time-wheel from occupying too much memory.
    // FIX: https://github.com/BinChengZhao/delay-timer/issues/28
    pub(crate) fn shrink(&mut self) {
        self.task_map.shrink_to(64);
    }
}
