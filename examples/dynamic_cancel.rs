use delay_timer::prelude::*;
use smol::Timer;
use std::time::Duration;
use std::thread::park_timeout;

// cargo run --package delay_timer --example dynamic_cancel --features=full

fn main() {
    let delay_timer = DelayTimerBuilder::default().build();
    let task_builder = TaskBuilder::default();

    let instance_chain = delay_timer.insert_task(build_task(task_builder)).unwrap();
    let instance_list = instance_chain.get_instance_list();
    park_timeout(Duration::from_secs(2));

    dbg!(instance_list.front());

}

fn build_task(mut task_builder: TaskBuilder) -> Task {
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
