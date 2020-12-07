use delay_timer::{
    delay_timer::DelayTimer,
    timer::task::{Frequency, Task, TaskBuilder},
    utils::functions::{create_default_delay_task_handler, create_delay_task_handler},
};
use hyper::{Client, Uri};

use std::thread::{current, park, Thread};

use delay_timer::timer::timer_core::get_timestamp;
//TODO: When you try to run that's example nedd add feature `tokio-support`.
use delay_timer::prelude::*;
use anyhow::Result;

//TODO: hyper 的依赖有问题，hyper目前是依赖tokio到0.2.23. 我本地跑的tokio是 0.3.*的，所以不兼容。
//TODO:cargo run --example=cycle_tokio_task --features=tokio-support

fn main() {
    let delay_timer = DelayTimer::new_with_tokio(None);
    let task_builder = TaskBuilder::default();
    delay_timer.add_task(build_task(task_builder)).unwrap();
    delay_timer.add_task(build_wake_task(task_builder)).unwrap();

    park();
    delay_timer.stop_delay_timer().unwrap();
}

fn build_task(mut task_builder: TaskBuilder) -> Task {
    let body = generate_closure_template("delay_timer is easy to use. .".into());

    //TODO:use candy.
    task_builder
        .set_frequency(Frequency::Repeated(
            "10,15,25,50 0/1 * * Jan-Dec * 2020-2100",
        ))
        .set_task_id(5)
        .set_maximum_running_time(15)
        .spawn(body)
        .unwrap()
}

fn build_wake_task(mut task_builder: TaskBuilder) -> Task {
    // let body = create_default_delay_task_handler;
    let thread: Thread = current();
    let body = move || {
        println!("bye bye");
        thread.unpark();
        create_default_delay_task_handler()
    };

    //TODO:use candy.
    task_builder
        .set_frequency(Frequency::Once("0 * * * Jan-Dec * 2020-2100"))
        .set_task_id(7)
        .set_maximum_running_time(50)
        .spawn(body)
        .unwrap()
}

pub fn generate_closure_template(
    name: String,
) -> impl Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
    move || {
        create_delay_task_handler(async_spawn(async_template(
            get_timestamp() as i32,
            name.clone(),
        )))
    }
}

pub async fn async_template(_id: i32, _name: String) -> Result<()> {
    //TODO:Optimize.
    // let url = format!("https://httpbin.org/get?id={}&name={}", id, name);
    let client = Client::new();
    //TODO:The default connector does not handle TLS.
    //Speaking to https destinations will require configuring a connector that implements TLS.
    //So use http for test.
    let uri: Uri = "http://baidu.com".parse().unwrap();
    dbg!(&uri);
    let res = client.get(uri).await?;
    println!("Response: {}", res.status());
    // Concatenate the body stream into a single buffer...
    let buf = hyper::body::to_bytes(res).await?;
    println!("body: {:?}", buf);
    Ok(())
}
