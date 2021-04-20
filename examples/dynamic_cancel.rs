use delay_timer::prelude::*;
use smol::Timer;
use tokio::runtime::Runtime;

use std::sync::Arc;
use std::time::Duration;
// cargo run --package delay_timer --example dynamic_cancel --features=full

fn main() {
    sync_cancel();
    println!("");
    async_cancel();
}

/// In Sync Context.
fn sync_cancel() {
    let delay_timer = DelayTimerBuilder::default().build();

    let instance_chain = delay_timer
        .insert_task(build_task(TaskBuilder::default()))
        .unwrap();

    instance_chain
        .next_with_wait()
        .unwrap()
        .cancel_with_wait()
        .unwrap();
}

/// In Async Context.
fn async_cancel() {
    let tokio_rt = Arc::new(Runtime::new().unwrap());
    let delay_timer = DelayTimerBuilder::default()
        .tokio_runtime(Some(tokio_rt.clone()))
        .build();

    tokio_rt.block_on(async {
        let task_builder = TaskBuilder::default();

        let task_instance_chain = delay_timer.insert_task(build_task(task_builder)).unwrap();
        let shell_task_instance_chain = delay_timer
            .insert_task(build_shell_task(task_builder))
            .unwrap();

        let task_instance = async {
            task_instance_chain
                .next_with_async_wait()
                .await
                .unwrap()
                .cancel_with_async_wait()
                .await
                .unwrap();
        };

        task_instance.await;
        let shell_task_instance = async {
            shell_task_instance_chain
                .next_with_async_wait()
                .await
                .unwrap()
                .cancel_with_async_wait()
                .await
                .unwrap();
        };

        shell_task_instance.await;
    });
}

fn build_task(mut task_builder: TaskBuilder) -> Task {
    let body = create_async_fn_body!({
        dbg!("create_async_fn_body!");
        Timer::after(Duration::from_secs(2)).await;
        dbg!("Never see this line.");
    });

    task_builder
        .set_task_id(1)
        .set_frequency(Frequency::Repeated("0/6 * * * * * *"))
        .set_maximun_parallel_runable_num(2)
        .spawn(body)
        .unwrap()
}

fn build_shell_task(mut task_builder: TaskBuilder) -> Task {
    let body = tokio_unblock_process_task_fn("php /home/open/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt".into());
    task_builder
        .set_frequency_by_candy(CandyFrequency::Repeated(CandyCron::Secondly))
        .set_task_id(3)
        .set_maximum_running_time(10)
        .set_maximun_parallel_runable_num(1)
        .spawn(body)
        .unwrap()
}
