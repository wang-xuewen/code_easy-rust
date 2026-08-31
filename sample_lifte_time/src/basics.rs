//! 生命周期基础：引用、借用与悬垂引用。
//!
//! 这是理解生命周期一切概念的起点。先分清三个东西：
//!
//! - **所有权**：值归谁所有，谁负责释放（`String`、`Vec` 拥有数据）。
//! - **借用**：`&T`（不可变借用）与 `&mut T`（可变借用），是「借用数据」的引用。
//! - **生命周期**：描述「借用」有效作用域的编译期概念，用 `'a` 这类标签表示。
//!
//! ## 一句话本质
//!
//! > 生命周期 = 编译器用来证明「引用不会悬垂」的契约。
//!
//! 当函数返回一个引用时，编译器无法从参数顺序判断返回值来自哪个入参，
//! 因此需要生命周期标注把「输入」与「输出」之间的存活关系明确写出来。

/// 返回两个字符串切片中较长的一个——生命周期最经典的教科书例子。
///
/// 标注 `'a` 表达了三件事：
/// - 输入 `x`、`y` 都至少活得和 `'a` 一样久；
/// - 返回值的生命周期是 `'a`；
/// - 由此编译器可以推断：返回值不会比 `x` 和 `y` 中**较短**的那个活得更久，
///   从而杜绝了「函数返回后，底层数据已被释放」的悬垂引用。
///
/// # 示例
///
/// ```
/// use lifetime_showcase::basics::longest;
///
/// let a = String::from("hello");
/// let b = "world!";
/// // a 与 b 都是借用传入，返回值借用二者中较长者
/// assert_eq!(longest(&a, b), "world!");
/// ```
pub fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() >= y.len() {
        x
    } else {
        y
    }
}

/// 演示：**生命周期标注不能凭空制造引用**。
///
/// 下面的代码是错误的（因此用 `compile_fail` 标注）：
/// 函数试图返回一个指向「局部变量 `result`」的引用，但 `result` 在函数
/// 返回时就被销毁了，这个引用会成为悬垂引用。
///
/// ```compile_fail
/// fn dangling() -> &'static str {
///     let result = String::from("temporary");
///     &result  // 错误：返回局部变量的引用
/// }
/// ```
///
/// 正确做法是「拥有者把数据一起交出来」，返回拥有所有权的 `String`：
///
/// ```
/// use lifetime_showcase::basics::make_owned;
/// let s = make_owned();
/// assert_eq!(s, "temporary");
/// ```
pub fn make_owned() -> String {
    String::from("temporary")
}

/// 演示：`&'static str` 是**字符串字面量**的默认类型。
///
/// 字符串字面量被硬编码进只读数据段（rodata），在整个程序运行期间都有效，
/// 因此其生命周期天然是 `'static`。这也是为什么下面的函数签名成立：
/// 字面量永远不会悬垂。
///
/// ```
/// use lifetime_showcase::basics::greet;
/// assert_eq!(greet(), "hello, lifetime");
/// ```
pub fn greet() -> &'static str {
    "hello, lifetime"
}

/// 演示：多个借用遵循 Rust 的借用规则。
///
/// 同一作用域内：
/// - 可以有任意多个不可变借用（`&T`）；
/// - 同一时刻**只能有一个**可变借用（`&mut T`）；
/// - 可变借用与不可变借用不能同时存在。
///
/// 生命周期标注让编译器能够精确计算每个借用何时开始、何时结束。
///
/// ```
/// use lifetime_showcase::basics::borrow_rules_demo;
/// assert_eq!(borrow_rules_demo(), 9); // "rust" + "acean" 共 9 字节
/// ```
pub fn borrow_rules_demo() -> i32 {
    let mut data = String::from("rust");

    // 一个不可变借用，作用域限制在这个块内（NLL 会精确追踪其最后使用点）
    {
        let r1 = &data;
        let _ = r1.len();
    }

    // 上一个不可变借用已结束，这里可以安全地取可变借用
    let r2 = &mut data;
    r2.push_str("acean");

    data.len() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_picks_longer() {
        let short = "ab";
        let long = String::from("abcdef");
        // short 借用 + long 借用，返回较长者
        assert_eq!(longest(short, &long), "abcdef");
    }

    #[test]
    fn longest_is_reference_not_copy() {
        let x = String::from("xxxxxxxxxx");
        let y = "yy";
        let r = longest(&x, y);
        // 返回值是引用，x 仍然拥有自己的数据
        assert_eq!(x, "xxxxxxxxxx");
        assert_eq!(r, "xxxxxxxxxx");
    }

    #[test]
    fn owned_string_is_moved_back() {
        assert_eq!(make_owned(), "temporary");
    }

    #[test]
    fn static_str_never_dangles() {
        assert_eq!(greet(), "hello, lifetime");
    }
}
