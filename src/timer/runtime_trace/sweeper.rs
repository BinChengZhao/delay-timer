use std::{
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd, Reverse},
    collections::BinaryHeap,
};

#[derive(Default, Eq)]
struct RecycleUnit {
    deadline: u64,
    task_id: usize,
    record_id: i64,
}

//TODO:reseve.
impl Ord for RecycleUnit {
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline.cmp(&other.deadline)
    }
}

impl PartialOrd for RecycleUnit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for RecycleUnit {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}

#[derive(Default)]
struct RecyclingBins {
    recycle_unit_heap: BinaryHeap<Reverse<RecycleUnit>>,
}

//when, event-handle update task_handle then matain recycle_unit_heap.
//TODO: communication by async-channel, t.recv().or(RecyclingBins.excute()).await;
//one by one run.....  maybe don't care time out.
impl RecyclingBins {
    pub(crate) fn begin_recycle(&mut self) -> Option<RecycleUnit> {
        //get now timestamp

        //pop one of recycle_unit_heap cmp if bigger than `now`, execute it.

        //until RecycleUnit is bigger than `now`

        //Maximum running time per run is 200ms.

        self.recycle_unit_heap.pop().and_then(|v| Some(v.0))
    }

    pub(crate) fn add_recycle_unit(&mut self, recycle_unit: RecycleUnit) {
        self.recycle_unit_heap.push(Reverse(recycle_unit));
    }

    pub(crate) fn execute_one(&mut self) {

        //if check is already timeout , should use RecycleUnit.task
    }

    //TODO: bind futures by or, can avoid timeout.
    //REF:https://docs.rs/smol/1.0.1/smol/struct.Timer.html
}
