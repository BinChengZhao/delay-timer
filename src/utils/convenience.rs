//! Convenience
//! It is a module that provides sugar-type and helper function.
use crate::prelude::*;

/// No size type, API compliant consistency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MyUnit;

impl DelayTaskHandler for MyUnit {
    fn quit(self: Box<Self>) -> AnyResult<()> {
        Ok(())
    }
}

/// The convenient functions to combine.
pub mod functions {

    use super::super::parse_and_run;
    use crate::prelude::*;
    use crate::timer::runtime_trace::task_handle::DelayTaskHandler;

    /// UnBlock execution of a command line task in delay-timer.
    #[instrument]
    pub async fn unblock_process_task_fn(shell_command: String) {
        use smol::process::{Child, Command};
        debug!("Unblock-Process Task start");

        let shell_command_clone = shell_command.clone();
        let childs = parse_and_run::<Child, Command>(&shell_command_clone).await;

        if let Err(err) = childs {
            return debug!("Unblock-Process Task fail for: {}", err.to_string());
        }

        let mut childs = childs.expect("");

        if let Ok(last_child) = childs.pop_back().ok_or_else(|| error!("Without child.")) {
            let _output = last_child.wait_with_output().await;

            // TODO: I can define a global variable `StatusReporter-Sender` and
            // Send `process-output` data to `StatusReporter` using the method that wraps the variable.
            //
            // context
            //     .finish_task(Some(FinishOutput::ProcessOutput(_output)))
            //     .await;
        }
    }

    use tokio::process::{Child, Command};

    /// UnBlock execution of a command line task in delay-timer `Runtime` based on tokio.
    #[instrument]
    pub fn tokio_unblock_process_task_fn(
        shell_command: String,
    ) -> impl Fn(TaskContext) -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
        move |context: TaskContext| {
            let shell_command_clone = shell_command.clone();
            create_delay_task_handler(async_spawn_by_tokio(async move {
                let childs = parse_and_run::<Child, Command>(&shell_command_clone).await;

                if let Err(err) = childs {
                    context
                        .finish_task(Some(FinishOutput::ExceptionOutput(err.to_string())))
                        .await;
                    return Err(anyhow!(err.to_string()));
                }

                let mut childs = childs?;

                let last_child = childs.pop_back().ok_or_else(|| anyhow!("Without child."))?;
                let output = last_child.wait_with_output().await?;
                context
                    .finish_task(Some(FinishOutput::ProcessOutput(output)))
                    .await;

                Ok(())
            }))
        }
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

/// cron expression syntax sugar related.
pub mod cron_expression_grammatical_candy {
    use std::ops::Deref;

    #[derive(Debug, Clone)]
    // Here, for the convenience of the user to create CandyCronStr,
    // it is the internal type of CandyCronStr that from &'static str is changed to String,
    // so that the user can construct CandyCronStr according to the indefinite conditions of the runtime.
    // For: https://github.com/BinChengZhao/delay-timer/issues/4
    /// Syntactic sugar enumeration of the corresponding structure instance.
    pub struct CandyCronStr(pub String);

    impl Deref for CandyCronStr {
        type Target = str;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[derive(Debug, Copy, Clone)]
    /// Syntactic sugar for cron expressions.
    pub enum CandyCron {
        /// This variant for Secondly.
        Secondly,
        /// This variant for Minutely.
        Minutely,
        /// This variant for Hourly.
        Hourly,
        /// This variant for Daily.
        Daily,
        /// This variant for Weekly.
        Weekly,
        /// This variant for Monthly.
        Monthly,
        /// This variant for Yearly.
        Yearly,
    }
    use CandyCron::*;

    impl From<CandyCron> for CandyCronStr {
        fn from(value: CandyCron) -> CandyCronStr {
            match value {
                Secondly => CandyCronStr(String::from("@secondly")),
                Minutely => CandyCronStr(String::from("@minutely")),
                Hourly => CandyCronStr(String::from("@hourly")),
                Daily => CandyCronStr(String::from("@daily")),
                Weekly => CandyCronStr(String::from("@weekly")),
                Monthly => CandyCronStr(String::from("@monthly")),
                Yearly => CandyCronStr(String::from("@yearly")),
            }
        }
    }

    #[derive(Debug, Copy, Clone)]
    ///Enumerated values of repeating types.
    pub enum CandyFrequency<T: Into<CandyCronStr>> {
        ///Repeat once.
        Once(T),
        ///Repeat ad infinitum.
        Repeated(T),
        ///Type of countdown.
        CountDown(u32, T),
    }
}

/// Provide a template function that supports dynamic generation of closures.
pub fn generate_closure_template(
    a: i32,
    b: String,
) -> impl Fn() -> Box<dyn DelayTaskHandler> + 'static + Send + Sync {
    move || {
        self::functions::create_delay_task_handler(async_spawn_by_smol(async_template(
            a,
            b.clone(),
        )))
    }
}

/// This is a demo case to demonstrate a custom asynchronous task.
pub async fn async_template(_: i32, _: String) -> AnyResult<()> {
    Ok(())
}

mod tests {

    #[test]
    fn test_cron_candy() {
        use super::cron_expression_grammatical_candy::{CandyCron, CandyCronStr};

        let mut s: String;

        s = <CandyCron as Into<CandyCronStr>>::into(CandyCron::Daily).0;
        assert_eq!(s, "@daily");

        s = <CandyCron as Into<CandyCronStr>>::into(CandyCron::Yearly).0;
        assert_eq!(s, "@yearly");

        s = <CandyCron as Into<CandyCronStr>>::into(CandyCron::Secondly).0;

        assert_eq!(s, "@secondly");
    }

    #[test]
    fn test_customization_cron_candy() {
        use super::cron_expression_grammatical_candy::CandyCronStr;
        use std::convert::Into;
        use std::ops::Deref;

        struct CustomizationCandyCron(i32);

        impl Into<CandyCronStr> for CustomizationCandyCron {
            fn into(self) -> CandyCronStr {
                let s = match self.0 {
                    0 => "1 1 1 1 1 1 1",
                    1 => "0 59 23 18 11 3 2100",
                    _ => "* * * * * * *",
                };
                CandyCronStr(s.to_owned())
            }
        }

        let mut candy_cron_str: CandyCronStr;

        candy_cron_str = CustomizationCandyCron(0).into();
        debug_assert_eq!(
            <CandyCronStr as Deref>::deref(&candy_cron_str),
            "1 1 1 1 1 1 1"
        );

        candy_cron_str = CustomizationCandyCron(1).into();
        debug_assert_eq!(candy_cron_str.deref(), "0 59 23 18 11 3 2100");

        candy_cron_str = CustomizationCandyCron(999).into();
        debug_assert_eq!(&*candy_cron_str, "* * * * * * *");
    }
}
