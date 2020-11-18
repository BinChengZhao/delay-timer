#[macro_use]
pub mod generate_fn_macro;
//TODO: macro_use 暴露宏后，就像文本一样已经把宏导入到当前mod了。
//#[macro_export] 是表示把宏 pub 了。
//记笔记。

pub(crate) mod feature_cfg;
pub use create_async_fn_body;
