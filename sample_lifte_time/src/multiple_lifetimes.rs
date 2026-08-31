//! 多个生命周期参数。
//!
//! 当函数或结构体有多个输入引用，且它们的存活时间可能不同时，就需要多个
//! 生命周期参数 `'a`、`'b`、`'c`……。这些参数彼此独立，编译器据此做更精确的
//! 检查——这正是生命周期标注的力量所在：**你能用参数表达引用之间的精确关系**。

/// 返回**第一个**参数的引用：返回值只与 `a` 绑定，与 `b` 完全无关。
///
/// 两个输入引用拥有独立生命周期 `'a` 与 `'b`。因为函数体只返回 `a`，
/// 所以签名只把 `'a` 放到返回类型上。调用方可以传两个存活时间完全不同的引用。
///
/// ```
/// use lifetime_showcase::multiple_lifetimes::first;
/// let a = String::from("AAA");
/// let b = String::from("BBB");
/// assert_eq!(first(&a, &b), "AAA");
/// ```
// 此处 `'b` 虽不参与返回值、本可省略，但刻意显式写出，
// 以强调「两个输入可以拥有彼此独立、互不干扰的生命周期」。
#[allow(clippy::needless_lifetimes)]
pub fn first<'a, 'b>(a: &'a str, _b: &'b str) -> &'a str {
    a
}

/// 返回**两个引用中较长的那个**：这要求两个输入共享同一生命周期。
///
/// 对比 `first`：这里返回值可能来自 `a` 也可能来自 `b`，编译器无法静态确定，
/// 因此必须让 `a` 与 `b` 都满足同一个生命周期 `'a`，返回类型也用 `'a`。
/// 这正是 [`crate::basics::longest`] 的完整语义。
///
/// ```
/// use lifetime_showcase::multiple_lifetimes::longest_of_two;
/// let a = String::from("abc");
/// let b = String::from("abcdef");
/// assert_eq!(longest_of_two(&a, &b), "abcdef");
/// ```
pub fn longest_of_two<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() {
        a
    } else {
        b
    }
}

/// 演示：**何时需要两个生命周期，何时一个就够**。
///
/// 判断标准很简单——看返回值到底可能来自哪个输入：
/// - 只来自某一个输入 → 那个输入单独一个生命周期即可，其它输入可独立；
/// - 可能来自任意一个输入 → 相关输入必须共享同一个生命周期。
///
/// 下面两个函数是「错误 vs 正确」的对照说明，用 `compile_fail` 展示错误写法：
///
/// ```compile_fail
/// // 错误：返回类型只有一个 'a，但 a 和 b 各自独立，编译器无法证明返回谁
/// fn broken<'a, 'b>(a: &'a str, b: &'b str) -> &'a str {
///     if a.len() > b.len() { a } else { b } // 可能返回 b，但签名说返回 &'a str
/// }
/// ```
pub struct _Doc;

/// 一个「日志行迭代器」示例：把多个生命周期参数用到生产场景。
///
/// `LogIter<'a, 'b>` 同时借用两条来源不同的数据（如「原始文本」和「过滤词」），
/// 二者生命周期独立。迭代器产出的每一项只借用 `'a`（原始文本），
/// 因此 `'b` 的存在不干扰 `'a` 的使用。
pub struct LogIter<'a, 'b> {
    lines: std::str::Lines<'a>,
    keyword: &'b str,
}

impl<'a, 'b> LogIter<'a, 'b> {
    /// 构造一个只保留含有关键词的行的迭代器。
    ///
    /// ```
    /// use lifetime_showcase::multiple_lifetimes::LogIter;
    ///
    /// let text = String::from("ERROR: boom\nINFO: ok\nERROR: again");
    /// let keyword = String::from("ERROR");
    /// let matches: Vec<&str> = LogIter::new(&text, &keyword).collect();
    /// assert_eq!(matches, vec!["ERROR: boom", "ERROR: again"]);
    /// ```
    pub fn new(text: &'a str, keyword: &'b str) -> Self {
        LogIter {
            lines: text.lines(),
            keyword,
        }
    }
}

impl<'a, 'b> Iterator for LogIter<'a, 'b> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        // 先复制出 keyword 的引用（生命周期 'b），再用 find 迭代 lines（'a）。
        // 返回值只借用 lines，与 keyword 的生命周期 'b 无关。
        let keyword = self.keyword;
        self.lines.by_ref().find(|line| line.contains(keyword))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_ignores_second_lifetime() {
        let a = String::from("keep");
        let b = String::from("independent");
        assert_eq!(first(&a, &b), "keep");
    }

    #[test]
    fn longest_shares_lifetime() {
        let a = String::from("xx");
        let b = String::from("yyyy");
        assert_eq!(longest_of_two(&a, &b), "yyyy");
    }

    #[test]
    fn log_iter_filters_by_keyword() {
        let text = String::from("WARN: x\nERROR: y\nINFO: z\nERROR: w");
        let keyword = String::from("ERROR");
        let got: Vec<&str> = LogIter::new(&text, &keyword).collect();
        assert_eq!(got, vec!["ERROR: y", "ERROR: w"]);
    }
}
