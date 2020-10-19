use delay_timer::utils::*;
use std::thread;
use std::time::Duration;

fn main() {
    let childs = parse_and_run(
        r"php /home/zhaobicheng/project/rust/repo/myself/delay_timer/examples/try_spawn.php >> ./try_spawn.txt",
    );
    println!("childs:{:?}", childs);
    thread::sleep(Duration::new(5, 0));
}
