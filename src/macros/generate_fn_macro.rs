//! generate_fn_macro contains a simple macro that is used to generate a closure function
//! that we use for task building.
//! (use this macro with the caveat that variables captured inside the block can be reused,
//! if you need to create closures dynamically,
//! refer to generate_closure_template in convenience mod).
/// Create a closure that return a DelayTaskHandel by macro.
use crate::prelude::*;
#[macro_export]
macro_rules! create_async_fn_body {
    ($async_body:block) => {
        |context: TaskContext| {
            let f = async move {
                let future_inner = async move { $async_body };

                future_inner.await;

                if let Some(timer_event_sender) = context.timer_event_sender {
                    timer_event_sender
                        .send(TimerEvent::FinishTask(context.task_id, context.record_id))
                        .await
                        .unwrap();
                }
            };
            let handle = async_spawn(f);
            create_delay_task_handler(handle)
        }
    };
}

cfg_tokio_support!(
    #[macro_export]
    macro_rules! create_async_fn_tokio_body {
        ($async_body:block) => {
            |context| {
                let handle = tokio_async_spawn(async move { $async_body });
                create_delay_task_handler(handle)
            }
        };
    }
);
