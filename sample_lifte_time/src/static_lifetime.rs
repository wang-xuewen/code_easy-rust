//! `'static` 生命周期。
//!
//! `'static` 是生命周期里最容易被误解的一个。先纠正两个常见误区：
//!
//! - ❌ `'static` **不是**「这个值真的活到程序结束」；
//! - ✅ `'static` **是**「这个值**可以**活到程序结束」，即它不持有任何
//!   短于 `'static` 的借用。编译器用它表达「没有借用负担，可以一直有效」。
//!
//! ## 两种常见的 `'static`
//!
//! 1. `&'static T`：一个引用，指向的数据存活到程序结束（如字符串字面量）。
//! 2. `T: 'static`：一个约束，表示类型 `T` 内部不包含任何非 `'static` 的借用，
//!    因此可以被安全地持有任意久（常用于线程 spawn、全局缓存等）。

/// 字符串字面量天然是 `&'static str`。
///
/// 字面量被写入可执行文件的只读数据段（rodata），程序运行期间始终有效。
///
/// ```
/// use lifetime_showcase::static_lifetime::literal;
/// assert_eq!(literal(), "I live as long as the program");
/// ```
pub fn literal() -> &'static str {
    "I live as long as the program"
}

/// 通过 `Box::leak` 把「堆上数据」转为 `&'static` 引用。
///
/// `Box::leak` 会放弃对 `Box` 的所有权，让内存**永久泄漏**，从而得到一个
/// 真正活到程序结束的 `&'static str`。它适用于「一次性初始化、之后只读」
/// 的配置类数据；滥用会造成内存泄漏。
///
/// ```
/// use lifetime_showcase::static_lifetime::leak_to_static;
/// let s = leak_to_static();
/// assert_eq!(s, "leaked");
/// ```
pub fn leak_to_static() -> &'static str {
    // 创建一个拥有所有权的 String，再泄漏它换取 'static 引用
    Box::leak("leaked".to_string().into_boxed_str())
}

/// 演示 `T: 'static` 约束：约束的是**类型**，而不是某个具体值的存活时间。
///
/// 下面函数只接受「不含任何短命借用」的类型。`String`、`i32`、`Vec<u8>`
/// 这类拥有所有权的类型都满足 `'static`；而 `&str`（一个引用）不满足。
///
/// ```
/// use lifetime_showcase::static_lifetime::spawn_safe;
/// spawn_safe(String::from("owned data")); // 拥有所有权 → 满足 'static
/// spawn_safe(42);
/// ```
///
/// ```compile_fail
/// use lifetime_showcase::static_lifetime::spawn_safe;
/// let local = String::from("borrowed");
/// spawn_safe(&local); // 错误：&String 是借用，不满足 T: 'static
/// ```
pub fn spawn_safe<T: 'static>(_value: T) {
    // 现实中这里常见于 std::thread::spawn 的参数：线程可能比当前作用域活得久，
    // 所以编译器要求闭包捕获的值满足 'static。
}

/// 对比实验：`'static` 引用可以被「缩短」成更短的生命周期（协变）。
///
/// 一个 `&'static str` 可以传给任何要求 `&'a str` 的函数，因为
/// `'static` 比任意 `'a` 都长。这也是为什么字面量在几乎所有需要
/// `&str` 的地方都能直接用。
///
/// ```
/// use lifetime_showcase::static_lifetime::use_short;
/// let s: &'static str = "hello";
/// use_short(s); // &'static str 自动协变为 &'a str
/// ```
// 此处 'a 刻意显式写出，用于强调「短生命周期」这一教学概念。
#[allow(clippy::needless_lifetimes)]
pub fn use_short<'a>(_r: &'a str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_is_static() {
        let s: &'static str = literal();
        assert_eq!(s, "I live as long as the program");
    }

    #[test]
    fn leak_produces_static_ref() {
        let a = leak_to_static();
        let b = leak_to_static();
        assert_eq!(a, b);
    }

    #[test]
    fn owned_types_satisfy_static_bound() {
        spawn_safe(vec![1, 2, 3]);
        spawn_safe("literal"); // 字面量是 &'static str，也满足
    }
}
