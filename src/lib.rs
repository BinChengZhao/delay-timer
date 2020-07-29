#![feature(split_inclusive)]
#![feature(str_strip)]
#![feature(drain_filter)]
// #[allow(dead_code)]
pub mod delay_timer;
pub mod timer;
pub mod utils;

#[macro_use]
extern crate lazy_static;

//TODO: 记录作用域

//       {1}  ，作用域无变量接受 里面返回值需要是 ()，非()不能编译
//let  a = {1} ; 有接受者能正常变异

// {return 1;} 则作为函数的返回值1
