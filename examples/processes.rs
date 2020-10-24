use delay_timer::{
    delay_timer::DelayTimer,
    timer::task::{Frequency, TaskBuilder},
    utils::functions::unblock_process_task_fn,
};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut delay_timer = DelayTimer::new();
    let  task_builder = TaskBuilder::default();
    let body = unblock_process_task_fn(r"php F:\rust\owner\delayTimer\examples\try_spawn.php >> F:\rust\owner\delayTimer\examples\try_spawn.txt".to_string());

    // misrepresentation
    // let body = create_process_task_fn(r"php F:\rust\owner\delayTimer\examples\try_spawn.php | dir >> \a\try_spawn.txt".to_string());

    let _task = task_builder
        .set_frequency(Frequency::CountDown(2, "0/50 * * * * * *"))
        .set_task_id(1)
        .set_maximum_running_time(5)
        .spawn(body);
    delay_timer.add_task(_task);

    sleep(Duration::new(90, 0));
    delay_timer.stop_delay_timer();
}
