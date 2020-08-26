//TODO: demo


let my_async_task = delay_task_body!(async {
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
        Timer::after(Duration::from_secs(1)).await;
    }
    Ok(())
});

//eq

let body = || {
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
            Timer::after(Duration::from_secs(1)).await;
        }
        Ok(())
    });
    println!("task-async-spwan");
    create_delay_task_handler(smol_task)
};




//custom Handler version


impl DelayTaskHandler for MyUnit {
    fn quit(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

let mut delay_timer = DelayTimer::new();
let mut task_builder = TaskBuilder::default();
let body = || {
    println!("task 1 ,1s run");
    create_delay_task_handler(MyUnit)

};



//default version
let body = || {
    println!("task 1 ,1s run");
    create_default_delay_task_handler()

};