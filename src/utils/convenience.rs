use self::functions::create_delay_task_handler;
use crate::async_spawn;
use crate::timer::runtime_trace::task_handle::DelayTaskHandler;
use anyhow::Result;
///No size type, API compliant consistency.
pub struct MyUnit;

impl DelayTaskHandler for MyUnit {
    fn quit(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

pub mod functions {

    use super::{super::parse_and_run, Result};
    use crate::timer::runtime_trace::task_handle::DelayTaskHandler;
    use crate::{async_spawn, unblock_spawn};
    pub fn unblock_process_task_fn(
        shell_command: String,
    ) -> impl Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
        move || {
            let shell_command_clone = shell_command.clone();
            create_delay_task_handler(async_spawn(async {
                unblock_spawn(move || parse_and_run(&shell_command_clone)).await
            }))
        }
    }

    #[inline(always)]
    ///Generate a closure from a string of shell commands that will generate a list of processes.
    pub fn create_process_task_fn(
        shell_command: String,
    ) -> impl Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
        move || {
            create_process_task(&shell_command).unwrap_or_else(|e| {
                println!("create-process:error:{}", e);
                create_default_delay_task_handler()
            })
        }
    }

    #[inline(always)]
    ///Generate a list of processes from a string of shell commands,
    ///and let it convert to a `DelayTaskHander`.
    pub fn create_process_task(shell_command: &str) -> Result<Box<dyn DelayTaskHandler>> {
        let process_linked_list = parse_and_run(shell_command)?;
        Ok(create_delay_task_handler(process_linked_list))
    }

    #[inline(always)]
    ///convert task_handler of impl DelayTaskHandler to a `Box<dyn DelayTaskHander>`.
    pub fn create_delay_task_handler(
        task_handle: impl DelayTaskHandler + 'static + Send + Sync,
    ) -> Box<dyn DelayTaskHandler> {
        Box::new(task_handle) as Box<dyn DelayTaskHandler>
    }

    #[inline(always)]
    ///Create a Box<dyn DelayTaskHandler> illusion.
    pub fn create_default_delay_task_handler() -> Box<dyn DelayTaskHandler> {
        create_delay_task_handler(super::MyUnit)
    }
}

pub mod cron_expression_grammatical_candy {

    #[derive(Debug, Copy, Clone)]
    pub enum CronCandy {
        Secondly,
        Minutely,
        Hourly,
        Daily,
        Weekly,
        Monthly,
        Yearly,
    }
    use CronCandy::*;

    impl From<CronCandy> for &'static str {
        fn from(cron_candy: CronCandy) -> Self {
            match cron_candy {
                Secondly => "@secondly",
                Minutely => "@minutely",
                Hourly => "@hourly",
                Daily => "@daily",
                Weekly => "@weekly",
                Monthly => "@monthly",
                Yearly => "@yearly",
            }
        }
    }
}

/// Provide a template function that supports dynamic generation of closures.
pub fn generate_closure_template(
    a: i32,
    b: String,
) -> impl Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
    move || create_delay_task_handler(async_spawn(async_template(a, b.clone())))
}

pub async fn async_template(_: i32, _: String) -> Result<()> {
    Ok(())
}
