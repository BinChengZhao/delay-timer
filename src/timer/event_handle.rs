use super::{
    runtime_trace::{
        sweeper::{RecycleUnit, RecyclingBins},
        task_handle::TaskTrace,
    },
    timer_core::{
        get_timestamp, AsyncSender, Slot, Task, TaskMark, TimerEvent, TimerEventReceiver,
        TimerEventSender, DEFAULT_TIMER_SLOT_COUNT,
    },
};

use anyhow::Result;
use std::sync::{
    atomic::{AtomicU64, Ordering::Acquire},
    Arc,
};
use waitmap::WaitMap;

use smol::{
    channel::{unbounded, Receiver, Sender, TryRecvError::*},
    future::{race, FutureExt},
    lock::Mutex,
};

//use `AcqRel::AcqRel` to store and load.....
pub(crate) type SencondHand = Arc<AtomicU64>;

pub(crate) type SharedTaskWheel = Arc<WaitMap<u64, Slot>>;
pub(crate) type SharedTaskFlagMap = Arc<WaitMap<u64, TaskMark>>;
// pub(crate) type SharedTaskTrace = Arc<TaskTrace>;

//TODO: use TaskMapHeader insted of these Sharedxxxx.....

//TaskTrace: use event mes update.
// remove Task, can't stop runing taskHandle, just though cancel or cancelAll with taskid.
// maybe cancelAll msg before last `update msg`  check the
// flag_map slotid with biggest task-slotid in trace, if has one delay, send a msg for recycleer
// let it to trash the last taskhandle.
pub(crate) struct EventHandle {
    wheel_queue: SharedTaskWheel,
    task_flag_map: SharedTaskFlagMap,
    second_hand: SencondHand,
    task_trace: TaskTrace,
    timer_event_receiver: TimerEventReceiver,
    status_report_sender: Option<Sender<i32>>,
    recycle_unit_sources_sender: Sender<RecycleUnit>,
}

impl EventHandle {
    pub(crate) fn new(
        wheel_queue: SharedTaskWheel,
        task_flag_map: SharedTaskFlagMap,
        second_hand: SencondHand,
        timer_event_receiver: TimerEventReceiver,
        timer_event_sender: TimerEventSender,
    ) -> Self {
        let status_report_sender: Option<AsyncSender<i32>> = None;
        let task_trace = TaskTrace::default();

        let (recycle_unit_sources_sender, recycle_unit_sources_reciver) =
            unbounded::<RecycleUnit>();

        let recycling_bins = Arc::new(RecyclingBins::new(
            recycle_unit_sources_reciver,
            timer_event_sender,
        ));

        smol::spawn(
            recycling_bins
                .clone()
                .add_recycle_unit()
                .race(recycling_bins.recycle()),
        )
        .detach();

        EventHandle {
            wheel_queue,
            task_flag_map,
            second_hand,
            task_trace,
            timer_event_receiver,
            status_report_sender,
            recycle_unit_sources_sender,
        }
    }

    pub(crate) fn set_status_report_sender(&mut self, status_report_sender: AsyncSender<i32>) {
        self.status_report_sender = Some(status_report_sender);
    }

    //_handle_event
    //TODO: Maybe can package that in a delayTimeTask or smolTask... Tinking....
    //if package smolTask for it, impl Futuren for this can auto-work.
    //TODO:When separate handle_event   ,do not need global TASKMAP also and Mutex..........
    pub(crate) async fn handle_event(&mut self) {
        while let Ok(event) = self.timer_event_receiver.recv().await {
            match event {
                TimerEvent::StopTimer => {
                    //TODO: DONE all of runing tasks.
                    //clear queue.
                    panic!("i'm stop")
                }
                TimerEvent::AddTask(task) => {
                    let task_mark = self.add_task(*task);
                    self.record_task_mark(task_mark);
                }
                TimerEvent::RemoveTask(task_id) => {
                    self.remove_task(task_id).await;
                }
                TimerEvent::CancelTask(task_id, record_id) => {
                    self.cancel_task(task_id, record_id);
                }

                TimerEvent::AppendTaskHandle(task_id, delay_task_handler_box) => {
                    //if has deadline, set recycle_unit.
                    if let Some(deadline) = delay_task_handler_box.get_end_time() {
                        let recycle_unit = RecycleUnit::new(
                            deadline,
                            delay_task_handler_box.get_task_id(),
                            delay_task_handler_box.get_record_id(),
                        );
                        self.recycle_unit_sources_sender
                            .send(recycle_unit)
                            .await
                            .unwrap_or_else(|e| println!("{}", e));
                    }

                    self.task_trace.insert(task_id, delay_task_handler_box);
                }

                TimerEvent::StopTask(_task_id) => todo!(),
            }
        }
    }

    //TODO:CancelTask, Is cancel once when task is running;
    //I should storage processChild in somewhere, When cancel event hanple i will kill child.

    //TODO:
    //cancel is exit running task.
    //stop is suspension of execution(set vaild).
    //user delete task , node should remove.

    //any `Task`  i can set `valid`  for that stop.

    //if get cancel signal is sync task like 'spwan process' i can kill that, async i can cancel.
    //I think i can save async/sync handel in TaskTrace.

    //TODO: self.buf Maybe need clear.

    //add task to wheel_queue  slot
    fn add_task(&mut self, mut task: Task) -> TaskMark {
        let second_hand = self.second_hand.load(Acquire);
        let exec_time: u64 = task.get_next_exec_timestamp();
        println!(
            "event_handle:task_id:{}, next_time:{}, get_timestamp:{}",
            task.task_id,
            exec_time,
            get_timestamp()
        );
        //TODO:exec_time IS LESS THAN TIMESTAMP.
        let time_seed: u64 = exec_time - get_timestamp() + second_hand;
        let slot_seed: u64 = time_seed % DEFAULT_TIMER_SLOT_COUNT;

        task.set_cylinder_line(time_seed / DEFAULT_TIMER_SLOT_COUNT);

        println!(
            "event_handle:task_id:{}, next_time:{}, slot_seed:{}",
            task.task_id, exec_time, slot_seed
        );

        //copu task_id
        let task_id = task.task_id;

        self.wheel_queue
            .get_mut(&slot_seed)
            .unwrap()
            .value_mut()
            .add_task(task);

        TaskMark::new(task_id, slot_seed)
    }

    pub(crate) fn record_task_mark(&mut self, task_mark: TaskMark) {
        self.task_flag_map.insert(task_mark.task_id, task_mark);
    }

    //TODO: addCountDown 限制，可能 remove 消息先消费，update slot后消费
    //用waitmap.wait
    pub(crate) async fn remove_task(&mut self, task_id: u64) -> Option<Task> {
        let task_mark = self.task_flag_map.get(&task_id)?;

        let slot_mark = task_mark.value().get_slot_mark();

        self.wheel_queue
            .get_mut(&slot_mark)
            .unwrap()
            .value_mut()
            .remove_task(task_id)
    }

    pub fn cancel_task(&mut self, task_id: u64, record_id: i64) -> Option<Result<()>> {
        self.task_trace.quit_one_task_handler(task_id, record_id)
    }

    pub(crate) fn init_task_wheel(slots_numbers: u64) -> SharedTaskWheel {
        let task_wheel = WaitMap::new();

        for i in 0..slots_numbers {
            task_wheel.insert(i, Slot::new());
        }

        Arc::new(task_wheel)
    }
}