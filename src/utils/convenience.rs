use crate::timer::runtime_trace::task_handle::DelayTaskHandler;

use anyhow::Result;

pub struct MyUnit;

impl DelayTaskHandler for MyUnit {
    fn quit(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

pub mod functions {

    use super::{super::parse_and_run, Result};
    use crate::timer::runtime_trace::task_handle::DelayTaskHandler;

    #[inline(always)]
    pub fn create_process_task_fn(
        shell_command: String,
    ) -> impl Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
        move || {
            create_process_task(&shell_command).unwrap_or_else(|e| {
                println!("{}", e);
                create_default_delay_task_handler()
            })
        }
    }

    #[inline(always)]
    pub fn create_process_task(shell_command: &str) -> Result<Box<dyn DelayTaskHandler>> {
        let process_linked_list = parse_and_run(shell_command)?;
        Ok(create_delay_task_handler(process_linked_list))
    }

    #[inline(always)]
    pub fn create_delay_task_handler(
        task_handle: impl DelayTaskHandler + 'static + Send + Sync,
    ) -> Box<dyn DelayTaskHandler> {
        Box::new(task_handle) as Box<dyn DelayTaskHandler>
    }

    #[inline(always)]
    pub fn create_default_delay_task_handler() -> Box<dyn DelayTaskHandler> {
        create_delay_task_handler(super::MyUnit)
    }
}
