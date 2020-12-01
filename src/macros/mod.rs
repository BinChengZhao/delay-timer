#[macro_use]
pub(crate) mod feature_cfg;

#[macro_use]
pub mod generate_fn_macro;

pub(crate) use cfg_smol_support;
pub(crate) use cfg_status_report;
pub(crate) use cfg_tokio_support;
pub use create_async_fn_body;

cfg_tokio_support!(
    pub use create_async_fn_tokio_body;
);
