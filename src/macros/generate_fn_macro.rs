//! generate_fn_macro contains a simple macro that is used to generate a closure function
//! that we use for task building.
//! (use this macro with the caveat that variables captured inside the block can be reused,
//! if you need to create closures dynamically,
//! refer to generate_closure_template in convenience mod).

/// Create a closure that return a DelayTaskHandel by macro.
#[macro_export]
macro_rules! create_async_fn_body {
    ($async_body:block) => {
        move |context: TaskContext| {
            let f = async move {
                let future_inner = async move { $async_body };
                future_inner.await;

                context.finishe_task().await;
            };
            let handle = async_spawn(f);
            create_delay_task_handler(handle)
        }
    };

    //FIXME: By std::concat_idents.
    // (($($capture_variable:ident),+) $async_body:block) => {

    //     move |context: TaskContext| {

    //         $(
    //             $capture_variable+"_ref" = $capture_variable.clone();
    //         )+
    //         let f = async move {
    //             let future_inner = async move { $async_body };
    //             future_inner.await;

    //             context.finishe_task().await;
    //         };
    //         let handle = async_spawn(f);
    //         create_delay_task_handler(handle)
    //     }
    // }
}

cfg_tokio_support!(
    #[macro_export]
    macro_rules! create_async_fn_tokio_body {
        ($async_body:block) => {
            |context| {
                let handle = tokio_async_spawn(async move {
                    let future_inner = async move { $async_body };
                    future_inner.await;

                    context.finishe_task().await;
                });
                create_delay_task_handler(handle)
            }
        };
    }
);
