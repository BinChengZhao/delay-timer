use std::process::Command;
use std::thread;
use std::time::Duration;
fn main() {
    let mut process = Command::new("php")
        .arg(r"F:\rust\owner\delayTimer\examples\try_spawn.php")
        .spawn()
        .expect("Failed to execute command");

    let thread = thread::spawn(|| {
        thread::sleep(Duration::from_secs(3));
        println!("i'm thread alive.")
    });

    // drop can't stop thread or process
    // just detach that.
    drop(process);
    drop(thread);

    println!("child is_already  gone");
    thread::sleep(Duration::new(5, 0));
    // I kill  process  when  it gone.
    // println!(
    //     "i'm kill process_child when child is gone {:?}",
    //     process.kill()
    // );
}

//https://doc.rust-lang.org/std/process/struct.Child.html
//process:
//Representation of a running or exited child process.

//This structure is used to represent and manage child processes. A child process is created via the Command struct, which configures the spawning process and can itself be constructed using a builder-style interface.

//There is no implementation of Drop for child processes, so if you do not ensure the Child has exited then it will continue to run, even after the Child handle to the child process has gone out of scope.

//Calling wait (or other functions that wrap around it) will make the parent process wait until the child has actually exited before continuing.

//https://doc.rust-lang.org/std/thread/struct.JoinHandle.html
//JoinHandle：

//A JoinHandle detaches the associated thread when it is dropped, which means that there is no longer any handle to thread and no way to join on it.

//A JoinHandle detaches the associated thread when it is dropped, which means that there is no longer any handle to thread and no way to join on it.
