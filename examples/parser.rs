use delay_timer::utils::*;
use std::thread;
use std::time::Duration;

fn main() {
    let childs = parse_and_run(
        r"php F:\rust\owner\delayTimer\examples\try_spawn.php >> F:\rust\owner\delayTimer\examples\try_spawn.txt",
    );
    println!("childs:{:?}", childs);
    thread::sleep(Duration::new(5, 0));
}
