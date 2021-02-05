use anyhow::Result;
use delay_timer::prelude::*;
use smol::Timer;
use std::thread::{current, park, sleep, Thread};
use std::time::Duration;
use surf;

// cargo run --package delay_timer --example demo --features=full

fn main() {
    let delay_timer = DelayTimerBuilder::default().enable_status_report().build();
    let task_builder = TaskBuilder::default();

    delay_timer.add_task(build_task1(task_builder)).unwrap();

    delay_timer.add_task(build_task2(task_builder)).unwrap();
    delay_timer.add_task(build_task3(task_builder)).unwrap();
    delay_timer.add_task(build_task5(task_builder)).unwrap();
    delay_timer.add_task(build_task7(task_builder)).unwrap();

    // Let's do someting about 2s.
    sleep(Duration::new(2, 1_000_000));

    let task1_record_id = filter_task_recodeid(&delay_timer, |&x| x.get_task_id() == 1).unwrap();
    delay_timer.cancel_task(1, task1_record_id).unwrap();
    delay_timer.remove_task(1).unwrap();

    delay_timer.add_task(build_wake_task(task_builder)).unwrap();
    park();
    delay_timer.stop_delay_timer().unwrap();
}

fn build_task1(mut task_builder: TaskBuilder) -> Task {
    let body = create_async_fn_body!({
        println!("create_async_fn_body!");

        Timer::after(Duration::from_secs(3)).await;

        println!("create_async_fn_body:i'success");
    });
    task_builder
        .set_task_id(1)
        .set_frequency_by_candy(CandyFrequency::Repeated(CandyCron::Secondly))
        .set_maximun_parallel_runable_num(2)
        .spawn(body)
        .unwrap()
}

fn build_task2(mut task_builder: TaskBuilder) -> Task {
    let body = create_async_fn_body!({
        let mut res = surf::get("https://httpbin.org/get").await.unwrap();
        dbg!(res.body_string().await.unwrap());
        Timer::after(Duration::from_secs(3)).await;
        dbg!("Task2 is done.");
    });
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::PerEightSeconds))
        .set_task_id(2)
        .set_maximum_running_time(5)
        .spawn(body)
        .unwrap()
}

fn build_task3(mut task_builder: TaskBuilder) -> Task {
    let body = unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt".into());
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(CandyCron::Minutely))
        .set_task_id(3)
        .set_maximum_running_time(5)
        .spawn(body)
        .unwrap()
}

fn build_task5(mut task_builder: TaskBuilder) -> Task {
    let body = generate_closure_template("delay_timer is easy to use. .".into());
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::LoveTime))
        .set_task_id(5)
        .set_maximum_running_time(5)
        .spawn(body)
        .unwrap()
}

fn build_task7(mut task_builder: TaskBuilder) -> Task {
    let body = create_async_fn_body!({
        dbg!(get_timestamp());

        Timer::after(Duration::from_secs(3)).await;
    });
    task_builder
        .set_task_id(7)
        .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousTime::PerDayFiveAclock))
        .set_maximun_parallel_runable_num(2)
        .spawn(body)
        .unwrap()
}

pub fn generate_closure_template(
    name: String,
) -> impl Fn(TaskContext) -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
    move |context| {
        let future_inner = async_template(get_timestamp() as i32, name.clone());

        let future = async move {
            future_inner.await.unwrap();
            context.finishe_task().await;
        };
        create_delay_task_handler(async_spawn(future))
    }
}

pub async fn async_template(id: i32, name: String) -> Result<()> {
    let url = format!("https://httpbin.org/get?id={}&name={}", id, name);
    let mut res = surf::get(url).await.unwrap();
    dbg!(res.body_string().await.unwrap());

    Ok(())
}

fn build_wake_task(mut task_builder: TaskBuilder) -> Task {
    let thread: Thread = current();
    let body = move |_| {
        println!("bye bye");
        thread.unpark();
        create_default_delay_task_handler()
    };

    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(CandyCron::Minutely))
        .set_task_id(700)
        .set_maximum_running_time(50)
        .spawn(body)
        .unwrap()
}

fn filter_task_recodeid<P>(delay_timer: &DelayTimer, predicate: P) -> Option<i64>
where
    P: FnMut(&PublicEvent) -> bool,
{
    let mut public_events = Vec::<PublicEvent>::new();

    while let Ok(public_event) = delay_timer.get_public_event() {
        public_events.push(public_event);
    }

    let public_event = public_events.into_iter().find(predicate)?;
    public_event.get_record_id()
}

enum AuspiciousTime {
    PerSevenSeconds,
    PerEightSeconds,
    LoveTime,
    PerDayFiveAclock,
}

impl Into<CandyCronStr> for AuspiciousTime {
    fn into(self) -> CandyCronStr {
        match self {
            Self::PerSevenSeconds => CandyCronStr("0/7 * * * * * *".to_string()),
            Self::PerEightSeconds => CandyCronStr("0/8 * * * * * *".to_string()),
            Self::LoveTime => CandyCronStr("0,10,15,25,50 0/1 * * Jan-Dec * 2020-2100".to_string()),
            Self::PerDayFiveAclock => CandyCronStr("01 00 1 * * * *".to_string()),
        }
    }
}
