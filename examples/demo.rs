use anyhow::Result;
use delay_timer::{
    delay_timer::DelayTimer,
    timer::{
        runtime_trace::task_handle::DelayTaskHandler,
        task::{Frequency, TaskBuilder},
    },
};
use smol::Task as SmolTask;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;
use surf;

fn main() {
    struct MyUnit;

    impl DelayTaskHandler for MyUnit {
        fn stop(&mut self) -> Result<()> {
            Ok(())
        }
    }

    let mut delay_timer = DelayTimer::new();
    let mut task_builder = TaskBuilder::default();
    let body = || {
        println!("task 1 ,1s run");
        Box::new(MyUnit) as Box<dyn DelayTaskHandler>
    };

    use std::fs::OpenOptions;
    use std::process::Command;

    //TODO:来一个简便函数，像thread::spawn() 一样，便捷生成任务。
    task_builder.set_frequency(Frequency::Repeated("* * * * * * *"));
    task_builder.set_task_id(1);
    let _task = task_builder.spawn(body);
    // delay_timer.add_task(_task);

    let mut task_builder = TaskBuilder::default();
    let body = || {
        let child = Command::new("php")
            .arg("-v")
            .spawn()
            .expect("Failed to execute command");
        Box::new(child) as Box<dyn DelayTaskHandler>
    };
    task_builder.set_frequency(Frequency::Repeated("0/5 * * * * * *"));
    task_builder.set_task_id(2);
    let _task = task_builder.spawn(body);
    // delay_timer.add_task(_task);

    let mut task_builder = TaskBuilder::default();
    let body = || {
        println!("task 3 ,3s run, altogether 3times");
        Box::new(MyUnit) as Box<dyn DelayTaskHandler>
    };
    task_builder.set_frequency(Frequency::CountDown(3, "0/3 * * * * * *"));
    task_builder.set_task_id(3);
    let _task = task_builder.spawn(body);
    delay_timer.add_task(_task);

    let mut task_builder = TaskBuilder::default();
    let body = || {
        println!("task 4 ,4s run, altogether 4times");
        Box::new(MyUnit) as Box<dyn DelayTaskHandler>
    };
    task_builder.set_frequency(Frequency::CountDown(4, "0/4 * * * * * *"));
    task_builder.set_task_id(3);
    let _task = task_builder.spawn(body);
    // delay_timer.add_task(task);

    let mut task_builder = TaskBuilder::default();
    let body = || {
        println!("async--spawn");
        let mut file = OpenOptions::new()
            .append(true)
            .write(true)
            .create(true)
            .open("./async.txt")
            .unwrap();
        file.write_all(b"hello").unwrap();

        let smol_task = SmolTask::spawn(async {

            for i in 1..10 {
                let s = format!("https://httpbin.org/get?id={}", i);
                SmolTask::spawn(async {
                    println!("{}", s);

                    let mut res = surf::get(s).await.unwrap();
                    let body_str = res.body_string().await.unwrap();
                    let mut file = OpenOptions::new()
                        .append(true)
                        .write(true)
                        .create(true)
                        .open("./async.txt")
                        .unwrap();
                    file.write_all(body_str.as_bytes()).unwrap();
                    ()
                })
                .detach();
            }

            // Ok(())
        }).detach();

        // Box::new(smol_task) as Box<dyn DelayTaskHandler>
        Box::new(MyUnit) as Box<dyn DelayTaskHandler>

        //TODO: Async Task should be fire by .await.
    };

    task_builder.set_frequency(Frequency::CountDown(5, "0/2 * * * * * *"));
    task_builder.set_task_id(5);
    let task = task_builder.spawn(body);

    delay_timer.add_task(task);
    loop {
        sleep(Duration::new(1, 0));
    }
}
