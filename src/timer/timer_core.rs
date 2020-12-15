use super::event_handle::SharedHeader;
pub(crate) use super::runtime_trace::task_handle::DelayTaskHandlerBox;
use super::runtime_trace::task_handle::DelayTaskHandlerBoxBuilder;
pub(crate) use super::slot::Slot;
pub(crate) use super::task::Task;
pub use crate::entity::get_timestamp;
use crate::prelude::*;
use snowflake::SnowflakeIdBucket;

pub(crate) use super::task::TaskMark;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::time::Duration;

pub(crate) const DEFAULT_TIMER_SLOT_COUNT: u64 = 3600;

pub(crate) type TimerEventSender = AsyncSender<TimerEvent>;
pub(crate) type TimerEventReceiver = AsyncReceiver<TimerEvent>;

cfg_tokio_support!(
    use tokio::time::{Interval, interval_at, self, Instant};

    #[derive(Debug)]
    struct Clock{
        inner : Interval
    }

    impl Clock{

        pub fn new(start: time::Instant, period: Duration) -> Self{
            let inner = interval_at(start, period);
            Clock{inner}
        }

        pub async fn tick(&mut self){
            self.inner.tick().await;
        }
    }

);

use futures::StreamExt;
use smol::Timer as smolTimer;
use std::time::Instant;

#[derive(Debug)]
struct Clock {
    inner: smolTimer,
    period: Duration,
}

impl Clock {
    pub fn new(start: Instant, period: Duration) -> Self {
        let inner = smolTimer::interval_at(start, period);
        Clock { inner, period }
    }

    pub async fn tick(&mut self) {
        self.inner.next().await;
    }
}

//warning: large size difference between variants
#[derive(Debug)]
pub enum TimerEvent {
    StopTimer,
    AddTask(Box<Task>),
    RemoveTask(u64),
    CancelTask(u64, i64),
    AppendTaskHandle(u64, DelayTaskHandlerBox),
}
#[derive(Clone)]
pub struct Timer {
    pub(crate) timer_event_sender: TimerEventSender,
    status_report_sender: Option<AsyncSender<i32>>,
    pub(crate) shared_header: SharedHeader,
}

//In any case, the task is not executed in the scheduler,
//and task-Fn determines which runtime to put the internal task in when it is generated.
//just provice api and struct ,less is more.
impl Timer {
    pub fn new(timer_event_sender: TimerEventSender, shared_header: SharedHeader) -> Self {
        Timer {
            timer_event_sender,
            status_report_sender: None,
            shared_header,
        }
    }

    //TODO:cfg macro
    #[cfg(feature = "status_report")]
    pub(crate) fn set_status_report_sender(&mut self, sender: AsyncSender<i32>) {
        self.status_report_sender = Some(sender);
    }

    ///Offset the current slot by one when reading it,
    ///so event_handle can be easily inserted into subsequent slots.
    pub(crate) fn next_position(&mut self) -> u64 {
        self.shared_header
            .second_hand
            .fetch_update(Release, Relaxed, |x| {
                Some((x + 1) % DEFAULT_TIMER_SLOT_COUNT)
            })
            .unwrap_or_else(|e| e)
    }

    ///Return a future can pool it for schedule all cycles task.
    pub(crate) async fn async_schedule(&mut self) {
        //if that overtime , i run it not block
        let mut second_hand;
        let mut timestamp;
        let mut clock = Clock::new(Instant::now(), Duration::from_secs(1));

        //TODO:auto-get nodeid and machineid.
        let mut snowflakeid_bucket = SnowflakeIdBucket::new(1, 1);
        loop {
            //TODO: replenish ending single, for stop current jod and thread.
            if !self.shared_header.shared_motivation.load(Acquire) {
                return;
            }

            second_hand = self.next_position();
            timestamp = get_timestamp();
            self.shared_header.global_time.store(timestamp, Release);
            let task_ids;

            {
                let mut slot_mut = self
                    .shared_header
                    .wheel_queue
                    .get_mut(&second_hand)
                    .unwrap();

                task_ids = slot_mut.value_mut().arrival_time_tasks();
            }

            for task_id in task_ids {
                let task_option: Option<Task>;

                {
                    let mut slot_mut = self
                        .shared_header
                        .wheel_queue
                        .get_mut(&second_hand)
                        .unwrap();

                    task_option = slot_mut.value_mut().remove_task(task_id);
                }

                if let Some(task) = task_option {
                    self.maintain_task(task, snowflakeid_bucket.get_id(), timestamp, second_hand)
                        .await;
                }
            }

            //FIXME: dependent logic, tokio / smol.
            clock.tick().await;
        }
    }

    pub(crate) async fn send_timer_event(
        &mut self,
        task_id: u64,
        tmp_task_handler_box: DelayTaskHandlerBox,
    ) {
        self.timer_event_sender
            .send(TimerEvent::AppendTaskHandle(task_id, tmp_task_handler_box))
            .await
            .unwrap_or_else(|e| println!("{}", e));
    }

    cfg_tokio_support!(
        pub(crate) async fn send_timer_event(
            &mut self,
            task_id: u64,
            tmp_task_handler_box: DelayTaskHandlerBox,
        ) {
            self.timer_event_sender
                .send(TimerEvent::AppendTaskHandle(task_id, tmp_task_handler_box))
                .unwrap_or_else(|e| println!("{}", e));
        }
    );

    #[inline(always)]
    pub async fn maintain_task(
        &mut self,
        mut task: Task,
        record_id: i64,
        timestamp: u64,
        second_hand: u64,
    ) -> Option<()> {
        let task_id = task.task_id;
        let task_handler_box = (task.get_body())();

        let delay_task_handler_box_builder = DelayTaskHandlerBoxBuilder::default();
        let tmp_task_handler_box = delay_task_handler_box_builder
            .set_task_id(task_id)
            .set_record_id(record_id)
            .set_start_time(timestamp)
            .set_end_time(task.get_maximum_running_time(timestamp))
            .spawn(task_handler_box);

        self.send_timer_event(task_id, tmp_task_handler_box).await;

        let task_valid = task.down_count_and_set_vaild();
        if !task_valid {
            return Some(());
        }
        //Next execute timestamp.
        let task_excute_timestamp = task.get_next_exec_timestamp();

        //Time difference + current second hand % DEFAULT_TIMER_SLOT_COUNT
        let step = task_excute_timestamp
            .checked_sub(timestamp)
            .unwrap_or_else(|| task.task_id % DEFAULT_TIMER_SLOT_COUNT)
            + second_hand;
        let quan = step / DEFAULT_TIMER_SLOT_COUNT;
        task.set_cylinder_line(quan);
        let slot_seed = step % DEFAULT_TIMER_SLOT_COUNT;

        {
            let mut slot_mut = self.shared_header.wheel_queue.get_mut(&slot_seed)?;

            slot_mut.value_mut().add_task(task);
        }

        {
            let mut task_flag_map = self.shared_header.task_flag_map.get_mut(&task_id)?;

            task_flag_map.value_mut().set_slot_mark(slot_seed);
        }
        Some(())
    }
}

mod tests {

    #[test]
    fn test_next_position() {
        use super::{SharedHeader, Timer, TimerEvent};
        use smol::channel::unbounded;
        use std::sync::atomic::Ordering;
        let (s, _) = unbounded::<TimerEvent>();
        let mut timer = Timer::new(s, SharedHeader::default());

        assert_eq!(timer.next_position(), 0);
        assert_eq!(timer.next_position(), 1);

        timer
            .shared_header
            .second_hand
            .store(3599, Ordering::SeqCst);
        assert_eq!(timer.next_position(), 3599);
        assert_eq!(timer.next_position(), 0);
    }
}
