//! # lifetime_showcase
//!
//! 一个**全方位展示 Rust 生命周期（lifetime）概念**的库。
//!
//! 本 crate 刻意不引入任何第三方依赖，用纯标准库 + 大量文档注释，
//! 通过一个贯穿始终的业务场景（**Markdown 文档分析器**）把生命周期的
//! 方方面面讲清楚。
//!
//! ## 生命周期是什么？
//!
//! 生命周期（lifetime）是 Rust 编译器（具体是「借用检查器 / borrow checker」）
//! 用来追踪**引用有效作用域**的机制。它回答的核心问题是：
//!
//! > 当某个引用被使用时，它指向的数据是否仍然存活？
//!
//! 关键心智模型（务必先建立）：
//! 1. **生命周期标注（`'a`）不会改变任何值的真实存活时间**，它只是给编译器
//!    提供一份「引用之间关系的契约」，让编译器据此检查而不是替你延长存活。
//! 2. 每一个引用 `&'a T` 都自带一个生命周期 `'a`，只是大多数时候编译器能
//!    依据「省略规则」自动推断，你无需手写。
//! 3. 生命周期只在「涉及引用」时出现。拥有所有权的值（`String`、`Vec` 等）
//!    不需要标注。
//!
//! ## 模块导航（按学习顺序推荐）
//!
//! | 模块 | 主题 |
//! |---|---|
//! | [`basics`] | 引用、借用、悬垂引用、经典 `longest` |
//! | [`elision`] | 生命周期省略规则（三条规则） |
//! | [`struct_lifetime`] | 结构体中持有引用时的生命周期 |
//! | [`methods`] | 方法、`impl<'a>` 与 `&self` 的生命周期 |
//! | [`multiple_lifetimes`] | 多个生命周期参数及其关系 |
//! | [`static_lifetime`] | `'static` 的含义与常见误解 |
//! | [`bounds`] | 生命周期约束（`T: 'a`）与 `where` 子句 |
//! | [`trait_object`] | trait 对象的生命周期（`Box<dyn Trait>`） |
//! | [`variance`] | 协变、逆变、不变（变体规则） |
//! | [`nll`] | 非词法作用域生命周期（NLL） |
//!
//! ## 快速开始
//!
//! ```no_run
//! use lifetime_showcase::basics::longest;
//!
//! let a = String::from("hello");
//! let b = "world!";
//! assert_eq!(longest(&a, b), "world!");
//! ```
//!
//! 或运行综合演示：
//! ```bash
//! cargo run --example showcase
//! ```

pub mod basics;
pub mod bounds;
pub mod elision;
pub mod methods;
pub mod multiple_lifetimes;
pub mod nll;
pub mod static_lifetime;
pub mod struct_lifetime;
pub mod trait_object;
pub mod variance;
