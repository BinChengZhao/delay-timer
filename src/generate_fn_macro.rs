#[macro_export]
/// Create a closure that return a DelayTaskHandel by macro.
macro_rules! create_async_fn_body {
    ($async_body:block) => {
        || {
            let handle = smol::spawn(async { $async_body });
            create_delay_task_handler(handle)
        }
    };
}
