//! This macro module provides the declaration macros used for the conditional compilation of lib,
//! and the helper macros provide cycle asynchronous tasks for the user.
#[macro_use]
pub(crate) mod feature_cfg;

#[macro_use]
pub mod generate_fn_macro;

pub use create_async_fn_body;

cfg_tokio_support!(
    pub use create_async_fn_tokio_body;
);
