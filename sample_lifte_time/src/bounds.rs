//! 生命周期约束（Outlives Bounds）：`T: 'a`、`'a: 'b` 与 `where` 子句。
//!
//! 除了给引用标注生命周期，你还可以用「约束」表达生命周期之间的**大小关系**：
//!
//! - `'a: 'b`：读作「`'a` 活得比 `'b` 久」（`'a` outlives `'b`）。
//! - `T: 'a`：类型 `T` 所包含的所有引用都至少活到 `'a`。
//! - `where` 子句：当约束较多时，用 `where` 写更清晰。

use std::fmt::Debug;

/// 约束 `'a: 'b`：声明 `'a` 比 `'b` 活得久，从而允许把 `'a`「缩短」成 `'b`。
///
/// 函数返回 `&'b str`，但函数体里可能返回 `&'a str` 的 `left`。编译器必须
/// 确信 `'a` 至少和 `'b` 一样长，才能安全地把 `&'a str` 当作 `&'b str` 返回。
/// 这就是 `where 'a: 'b` 的作用。
///
/// ```
/// use lifetime_showcase::bounds::choose_longer;
/// let a = String::from("longer-one");
/// let b = String::from("short");
/// // 这里 'a（a 的生命周期）确实比 'b 长
/// assert_eq!(choose_longer(&a, &b), "longer-one");
/// ```
pub fn choose_longer<'a, 'b>(left: &'a str, right: &'b str) -> &'b str
where
    'a: 'b,
{
    if left.len() >= right.len() {
        left // &'a str 被安全地当作 &'b str
    } else {
        right
    }
}

/// 约束 `T: 'a`：声明泛型 `T` 不包含比 `'a` 更短的借用。
///
/// 下面的结构体 `Cache<'a, T>` 把「生命周期 `'a`」和「数据 `T`」绑在一起：
/// 它保证 `T` 里的任何引用都至少活到 `'a`，这样缓存对象整体可以安全地
/// 活到 `'a`。
///
/// ```
/// use lifetime_showcase::bounds::Cache;
/// let key = String::from("k");
/// let cache = Cache::new(&key, 42usize);
/// assert_eq!(*cache.value(), 42);
/// ```
pub struct Cache<'a, T: 'a> {
    _key: &'a str,
    value: T,
}

impl<'a, T: 'a> Cache<'a, T> {
    pub fn new(key: &'a str, value: T) -> Self {
        Cache { _key: key, value }
    }

    pub fn value(&self) -> &T {
        &self.value
    }
}

/// 综合示例：用 `where` 子句同时约束「生命周期」和「trait 边界」。
///
/// 生产代码里 `where` 子句是让复杂签名保持可读的关键工具。
/// 这里 `T: 'a + Debug` 表示 T 既满足生命周期约束，又实现了 `Debug`。
///
/// ```
/// use lifetime_showcase::bounds::debug_in_scope;
/// let label = String::from("label");
/// debug_in_scope(&label, 99usize);
/// ```
pub fn debug_in_scope<'a, T>(_anchor: &'a str, value: T)
where
    T: 'a + Debug,
{
    // 实际代码里可能把 value 存到绑定 'a 的结构里，这里仅演示签名
    let _ = value;
}

/// 常见编译错误对照：缺少 `T: 'a` 约束时，编译器会拒绝。
///
/// 当你把泛型 `T` 放进一个要求「活到 `'a`」的容器（如 `Box<dyn Trait + 'a>`）
/// 时，编译器必须确认 `T` 内部不含短于 `'a` 的借用——否则无法保证容器
/// 整体能活到 `'a`。
///
/// ```compile_fail
/// use std::fmt::Debug;
/// // 错误：T 没有 T: 'a 约束，无法证明它能放进 Box<dyn Debug + 'a>
/// fn box_it<'a, T: Debug>(t: T) -> Box<dyn Debug + 'a> {
///     Box::new(t)
/// }
/// ```
pub struct _Doc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_longer_returns_right_lifetime() {
        let a = String::from("aaaa");
        let b = String::from("bb");
        // 'a（a）比 'b（b）长，约束成立
        assert_eq!(choose_longer(&a, &b), "aaaa");
    }

    #[test]
    fn cache_holds_value() {
        let key = String::from("k");
        let cache = Cache::new(&key, String::from("payload"));
        assert_eq!(cache.value(), "payload");
    }
}
