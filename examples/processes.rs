use delay_timer::{
    create_async_fn_body,
    delay_timer::DelayTimer,
    timer::{
        runtime_trace::task_handle::DelayTaskHandler,
        task::{Frequency, TaskBuilder},
    },
    utils::functions::{
        create_default_delay_task_handler, create_delay_task_handler, create_process_task_fn,
    },
};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut delay_timer = DelayTimer::new();
    let mut task_builder = TaskBuilder::default();
    let body = create_process_task_fn(r"php F:\rust\owner\delayTimer\examples\try_spawn.php >> F:\rust\owner\delayTimer\examples\try_spawn.txt".to_string());

    task_builder.set_frequency(Frequency::Repeated("* 0/1 * * * * *"));
    task_builder.set_task_id(1);
    task_builder.set_maximum_running_time(5);

    let _task = task_builder.spawn(body);
    delay_timer.add_task(_task);

    loop {
        sleep(Duration::new(1, 0));
    }
}
