use super::utils::functions::create_delay_task_handler;

#[macro_export]
macro_rules! create_async_fn_body {
    ($async_body:block) => {
        || {
            let handle = smol::spawn(async { $async_body });
            create_delay_task_handler(handle)
        }
    };
}
