use delay_timer::prelude::*;
use hyper::{Client, Uri};
use std::ops::Deref;
use std::thread::{current, park, Thread};

//When you try to run that's example nedd add feature `tokio-support`.
//cargo run --example=cycle_tokio_task --features=tokio-support

fn main() {
    let delay_timer = DelayTimerBuilder::default().tokio_runtime(None).build();
    let task_builder = TaskBuilder::default();
    delay_timer.add_task(build_task(task_builder)).unwrap();
    delay_timer.add_task(build_wake_task(task_builder)).unwrap();

    park();
    delay_timer.stop_delay_timer().unwrap();
}

fn build_task(mut task_builder: TaskBuilder) -> Task {
    let body = generate_closure_template("'delay_timer-is-easy-to-use.'".into());

    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousDay::Work))
        .set_task_id(5)
        .set_maximum_running_time(15)
        .spawn(body)
        .unwrap()
}

fn build_wake_task(mut task_builder: TaskBuilder) -> Task {
    let thread: Thread = current();
    let body = move |_| {
        println!("bye bye");
        thread.unpark();
        create_default_delay_task_handler()
    };

    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(AuspiciousDay::Wake))
        .set_task_id(7)
        .set_maximum_running_time(50)
        .spawn(body)
        .unwrap()
}

pub fn generate_closure_template(
    name: String,
) -> impl Fn(TaskContext) -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
    move |context| {
        let future_inner = async_template(get_timestamp() as i32, name.clone());

        let future = async move {
            future_inner.await;
            context.finishe_task().await;
        };

        create_delay_task_handler(async_spawn_by_tokio(future))
    }
}

pub async fn async_template(id: i32, name: String) {
    let client = Client::new();

    //The default connector does not handle TLS.
    //Speaking to https destinations will require configuring a connector that implements TLS.
    //So use http for test.
    let url = format!("http://httpbin.org/get?id={}&name={}", id, name);
    let uri: Uri = url.parse().unwrap();

    let res = client.get(uri).await.unwrap();
    println!("Response: {}", res.status());
    // Concatenate the body stream into a single buffer...
    let buf = hyper::body::to_bytes(res).await.unwrap();
    println!("body: {:?}", buf);
}

enum AuspiciousDay {
    Work,
    Wake,
}

impl Into<CandyCronStr> for AuspiciousDay {
    fn into(self) -> CandyCronStr {
        match self {
            Self::Work => CandyCronStr("10,15,25,50 0/1 * * Jan-Dec * 2020-2100"),
            Self::Wake => CandyCronStr("0 * * * Jan-Dec * 2020-2100"),
        }
    }
}
