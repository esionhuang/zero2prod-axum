/*
 * 利用集成测试中每个文件都会被编译成一个可执行文件的特性,将针对各个模块的测试拆分为多个文件
 * 不用显示编写 main 函数,rust会隐式实现一个
 */
pub mod admin_dashboard;
pub mod change_password;
pub mod health_check;
pub mod helpers;
pub mod login;
pub mod newsletter;
pub mod subscriptions;
pub mod subscriptions_confirm;

pub use helpers::*;
