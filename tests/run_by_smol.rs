use delay_timer::timer::timer::get_timestamp;
use futures::future;
use smol::{Task, Timer};
use std::{
    thread,
    time::{Duration, Instant},
};

#[test]
fn it_works() {
    //TODO: ONE THREAD , IS  serial.（一个线程的情况）
    //TODO: sleep let it block, i should try aother waiting method.
    //（thread::sleep，会阻塞执行的工作线程）

    // No need to `run()`, now we can just block on the main future.
    smol::run(async {
        let task = Task::spawn(async {
            thread::sleep(Duration::new(2, 0));
            println!("thread::sleep  会阻塞执行线程!{}", get_timestamp());
        });

        let tasg = Task::spawn(async {
            thread::sleep(Duration::new(2, 0));
            println!("2222222222222222222222222222222222:{}", get_timestamp());
        });

        future::join(task, tasg).await;
    })
}

#[test]
fn its_works() {
    //TODO: ONE THREAD , IS  serial.（同样，一个线程的情况）
    //TODO: sleep let it block, i should try aother waiting method.
    //（async fn sleep， 不会阻塞线程，因为他是一个状态机，poll了一下，没到时间就poll下一个）

    // No need to `run()`, now we can just block on the main future.
    smol::run(async {
        async fn sleep(dur: Duration) {
            Timer::after(dur).await;
        }
        let task = Task::spawn(async {
            sleep(Duration::from_secs(2)).await;
            println!("async fn sleep  并不会阻塞执行器的线程!{}", get_timestamp());
        });

        let tasg = Task::spawn(async {
            sleep(Duration::from_secs(2)).await;
            println!("2222222222222222222222222222222222:{}", get_timestamp());
        });

        future::join(task, tasg).await;
    })
}

#[test]
fn that_works() {
    //TODO: TWO THREAD , IS  divide equally .
    //两个线程去跑，一个阻塞了，另一个不阻塞的线程拿到另一个任务也去阻塞跑

    // A pending future is one that simply yields forever.
    thread::spawn(|| smol::run(future::pending::<()>()));

    // No need to `run()`, now we can just block on the main future.
    smol::run(async {
        let task = Task::spawn(async {
            thread::sleep(Duration::new(2, 0));
            println!("咱俩一块阻塞跑!{}", get_timestamp());
        });

        let tasg = Task::spawn(async {
            thread::sleep(Duration::new(2, 0));
            println!("2222222222222222222222222222222222:{}", get_timestamp());
        });

        future::join(task, tasg).await;
    });
}
