//! generate_fn_macro contains a simple macro that is used to generate a closure function
//! that we use for task building.
//! (use this macro with the caveat that variables captured inside the block can be reused,
//! if you need to create closures dynamically,
//! refer to generate_closure_template in convenience mod).
#[macro_export]
/// Create a closure that return a DelayTaskHandel by macro.
macro_rules! create_async_fn_body {
    ($async_body:block) => {
        || {
            let handle = delay_timer::async_spawn(async { $async_body });
            create_delay_task_handler(handle)
        }
    };
}
