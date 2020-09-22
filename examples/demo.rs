use anyhow::Result;
use delay_timer::{
    create_async_fn_body,
    delay_timer::DelayTimer,
    timer::{
        runtime_trace::task_handle::DelayTaskHandler,
        task::{Frequency, TaskBuilder},
    },
    utils::functions::create_delay_task_handler,
};
use smol::Timer;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use surf;

fn main() {
    struct MyUnit;

    impl DelayTaskHandler for MyUnit {
        fn quit(self: Box<Self>) -> Result<()> {
            Ok(())
        }
    }

    let mut delay_timer = DelayTimer::new();
    let mut task_builder = TaskBuilder::default();
    let body = || {
        println!("task 1 ,1s run");
        Box::new(MyUnit) as Box<dyn DelayTaskHandler>
    };

    //TODO:来一个简便函数，像thread::spawn() 一样，便捷生成任务。
    task_builder.set_frequency(Frequency::Repeated("* * * * * * *"));
    task_builder.set_task_id(1);
    let _task = task_builder.spawn(body);
    // delay_timer.add_task(_task);

    let mut task_builder = TaskBuilder::default();
    let body = || {
        println!("example:demo:php:run");

        let child = Command::new("php")
            .arg(r"F:\rust\owner\delayTimer\examples\try_spawn.php")
            .spawn()
            .expect("Failed to execute command");

        create_delay_task_handler(child)
    };
    task_builder.set_frequency(Frequency::Repeated("0/5 * * * * * *"));
    task_builder.set_task_id(2);
    let _task = task_builder.spawn(body);
    // delay_timer.add_task(_task);

    let mut task_builder = TaskBuilder::default();
    let body = || {
        println!("task 3 ,3s run, altogether 3times");
        create_delay_task_handler(MyUnit)
    };
    task_builder.set_frequency(Frequency::CountDown(3, "0/3 * * * * * *"));
    task_builder.set_task_id(3);
    let _task = task_builder.spawn(body);
    // delay_timer.add_task(_task).unwrap();

    let mut task_builder = TaskBuilder::default();
    let body = || {
        println!("task 4 ,4s run, altogether 4times");
        create_delay_task_handler(MyUnit)
    };
    task_builder.set_frequency(Frequency::CountDown(4, "0/4 * * * * * *"));
    task_builder.set_task_id(3);
    let _task = task_builder.spawn(body);
    // delay_timer.add_task(task);

    // let mut task_builder = TaskBuilder::default();
    // let body = || {
    //     let smol_task = SmolTask::spawn(async {
    //         for i in 1..10 {
    //             let s = format!("https://httpbin.org/get?id={}", i);
    //             SmolTask::spawn(async {
    //                 println!("{}", s);

    //                 let mut res = surf::get(s).await.unwrap();
    //                 let body_str = res.body_string().await.unwrap();
    //                 let mut file = OpenOptions::new()
    //                     .append(true)
    //                     .write(true)
    //                     .create(true)
    //                     .open("./async.txt")
    //                     .unwrap();
    //                 file.write_all(body_str.as_bytes()).unwrap();
    //                 ()
    //             })
    //             .detach();
    //             Timer::after(Duration::from_secs(1)).await;
    //         }
    //         Ok(())
    //     });
    //     println!("task-async-spwan");
    //     create_delay_task_handler(smol_task)
    // };

    // task_builder.set_frequency(Frequency::CountDown(5, "0/2 * * * * * *"));
    // task_builder.set_task_id(5);
    // let task = task_builder.spawn(body);

    // delay_timer.add_task(task).unwrap();

    let mut task_builder = TaskBuilder::default();
    let body = create_async_fn_body!({
        println!("create_async_fn_body!--7");
        Timer::after(Duration::from_secs(2)).await;

        println!("create_async_fn_body:i'm part of success--1");


        Timer::after(Duration::from_secs(2)).await;

        println!("create_async_fn_body:i'm part of success--2");

        Timer::after(Duration::from_secs(2)).await;


        println!("create_async_fn_body:i'success");
        Ok(())
    });
    task_builder.set_frequency(Frequency::CountDown(2, "0/7 * * * * * *"));
    task_builder.set_task_id(7);
    task_builder.set_maximum_running_time(5);
    let _task = task_builder.spawn(body);
    delay_timer.add_task(_task);

    loop {
        //infact loop is always run wait client send task-event.
        sleep(Duration::new(1, 0));
        // delay_timer.cancel_task(5, 25);
    }
}
