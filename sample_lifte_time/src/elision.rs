//! 生命周期省略规则（Lifetime Elision）。
//!
//! Rust 允许在很多场景下**省略**生命周期标注。省略并不意味着「没有生命周期」，
//! 而是编译器会按一套固定的规则自动补齐。理解这套规则，你才知道什么时候
//! 必须手写标注、什么时候可以省略。
//!
//! ## 三条省略规则
//!
//! 编译器在处理函数/方法签名时会依次应用：
//!
//! 1. **每个引用参数**各自获得一个独立生命周期（`fn f(x: &str)` → `fn f<'a>(x: &'a str)`）。
//! 2. 如果**恰好只有一个**输入生命周期，则它被赋给**所有**输出生命周期
//!    （`fn f(x: &str) -> &str` → `fn f<'a>(x: &'a str) -> &'a str`）。
//! 3. 如果方法含有 `&self` 或 `&mut self`，则 **`self` 的生命周期被赋给所有
//!    输出生命周期**（这解释了为什么大量方法无需标注即可返回引用）。
//!
//! 三条规则都用完仍无法推断某个输出引用的生命周期时，编译器会报错，
//! 要求你显式标注。

/// 规则 1 + 规则 2 的体现：一个输入引用 + 一个输出引用。
///
/// 只有一个入参引用，编译器直接把它（唯一）的生命周期赋给返回值，
/// 因此下面的签名无需手写任何 `'a`：
///
/// ```
/// use lifetime_showcase::elision::first_word;
/// assert_eq!(first_word("hello world"), "hello");
/// assert_eq!(first_word("rust"), "rust");
/// assert_eq!(first_word(""), "");
/// ```
pub fn first_word(s: &str) -> &str {
    // 这就是省略规则推断出的 `fn first_word<'a>(s: &'a str) -> &'a str`
    s.split_whitespace().next().unwrap_or("")
}

/// 两个输入引用、一个输出引用——省略规则**无法**判断返回值该借用谁，
/// 因此必须显式标注（这里 `'a` 是我们手写的）。
///
/// 这正是 [`crate::basics::longest`] 的场景：两条输入生命周期相同，
/// 编译器不知道返回值跟 `x` 还是 `y` 绑定，所以规则 2 不适用，必须标注。
///
/// ```
/// use lifetime_showcase::elision::pick_left;
/// let a = String::from("left");
/// let b = String::from("right");
/// assert_eq!(pick_left(&a, &b), "left");
/// ```
pub fn pick_left<'a>(x: &'a str, _y: &str) -> &'a str {
    x
}

/// 规则 3 的体现：`&self` 方法返回引用时自动绑定到 `self` 的生命周期。
///
/// 下面的 `Reader` 结构体持有 `&'a str`，其方法 `line` 返回 `&self` 内部的引用。
/// 方法签名里没有出现任何 `'a`，但编译器按规则 3 自动补全成：
/// `fn line(&'s self) -> &'s str`（其中 `'s` 是 `self` 的借用生命周期）。
///
/// ```
/// use lifetime_showcase::elision::Reader;
/// let text = String::from("one\ntwo\nthree");
/// let reader = Reader::new(&text);
/// assert_eq!(reader.line(0), "one");
/// assert_eq!(reader.line(1), "two");
/// ```
pub struct Reader<'a> {
    data: &'a str,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a str) -> Self {
        Reader { data }
    }

    /// 省略规则 3：返回值生命周期 = `&self` 的生命周期。
    pub fn line(&self, index: usize) -> &str {
        self.data.lines().nth(index).unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_word_handles_edge_cases() {
        assert_eq!(first_word("  hello   world"), "hello");
        assert_eq!(first_word("single"), "single");
        assert_eq!(first_word("   "), "");
    }

    #[test]
    fn pick_left_returns_left_reference() {
        let a = String::from("AAA");
        let b = String::from("BBBB");
        let r = pick_left(&a, &b);
        assert_eq!(r, "AAA");
    }

    #[test]
    fn reader_lines_track_source() {
        let text = String::from("l1\nl2\nl3");
        let reader = Reader::new(&text);
        assert_eq!(reader.line(0), "l1");
        assert_eq!(reader.line(2), "l3");
        assert_eq!(reader.line(99), "");
    }
}
