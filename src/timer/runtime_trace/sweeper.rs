use std::{collections::{BinaryHeap}, cmp::{Ordering, PartialOrd, Ord, PartialEq, Eq}};

#[derive(Default, Eq)]
struct RecycleUnit{
    deadline: i32,
    task_id: usize,
    record_id: i64
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
struct RecyclingBins{
    recycle_unit_heap:BinaryHeap<RecycleUnit>,
}

impl RecyclingBins {

    pub(crate) fn begin_recycle(&mut self){
        //get now timestamp

        //pop one of recycle_unit_heap cmp if bigger than `now`, execute it.

        //until RecycleUnit is bigger than `now`

        //Maximum running time per run is 200ms.
    } 

}