use crate::timer::runtime_trace::task_handle::DelayTaskHandler;

use anyhow::Result;

pub struct MyUnit;

impl DelayTaskHandler for MyUnit {
    fn quit(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

pub mod functions {

    use crate::timer::runtime_trace::task_handle::DelayTaskHandler;

    #[inline(always)]
    pub fn create_delay_task_handler(
        task_handle: impl DelayTaskHandler + 'static + Send + Sync,
    ) -> Box<dyn DelayTaskHandler> {
        Box::new(task_handle) as Box<dyn DelayTaskHandler>
    }

    pub fn create_default_delay_task_handler() -> Box<dyn DelayTaskHandler> {
        create_delay_task_handler(super::MyUnit)
    }
}
