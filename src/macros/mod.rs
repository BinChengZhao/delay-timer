#[macro_use]
pub(crate) mod feature_cfg;

#[macro_use]
pub mod generate_fn_macro;

pub use create_async_fn_body;

cfg_tokio_support!(
    pub use create_async_fn_tokio_body;
);
