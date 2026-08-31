//! 结构体中的生命周期。
//!
//! 当一个结构体**持有引用**（而不是拥有数据）时，编译器必须知道这些引用
//! 相对结构体本身能活多久，因此结构体定义上必须带生命周期参数。
//!
//! ## 核心规则
//!
//! > 结构体字段只要是引用，结构体就必须声明对应的生命周期，并把它「透传」给字段。
//!
//! 常见的两条设计取舍：
//! - **借用数据**（`&'a str`）：零拷贝、高效，但结构体的存活受限于数据源；
//! - **拥有数据**（`String`）：结构体独立存活、可移动、可返回，但有分配开销。

use std::fmt;

/// 一个「借用数据」的 Markdown 文档视图。
///
/// `Document<'a>` 不拥有 `title` / `body` 的字符串，只是引用它们。
/// 生命周期 `'a` 的含义：**这个 `Document` 实例的存活时间不能超过
/// 它借用的 `title` 与 `body` 的存活时间**。
///
/// # 示例
///
/// ```
/// use lifetime_showcase::struct_lifetime::Document;
///
/// let title = String::from("生命周期入门");
/// let body = String::from("# 什么是生命周期\n...");
/// let doc = Document::new(&title, &body);
/// assert_eq!(doc.title, "生命周期入门");
/// ```
pub struct Document<'a> {
    pub title: &'a str,
    pub body: &'a str,
}

impl<'a> Document<'a> {
    /// 构造一个借用 `title` 与 `body` 的文档视图。
    pub fn new(title: &'a str, body: &'a str) -> Self {
        Document { title, body }
    }

    /// 返回一个**拥有所有权**的摘要字符串。
    ///
    /// 注意返回类型是 `String` 而不是 `&str`：因为我们不想把返回值
    /// 绑定到 `self` 的存活上，而是「新造」一份数据交给调用方，
    /// 调用方可以自由持有、移动这个 `String`。
    pub fn summary(&self) -> String {
        let preview: String = self.body.chars().take(30).collect();
        format!("《{}》: {}...", self.title, preview)
    }
}

impl fmt::Display for Document<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "# {}\n\n{}", self.title, self.body)
    }
}

/// 演示：结构体持有引用却**不标注**生命周期，编译器会拒绝。
///
/// ```compile_fail
/// struct BrokenDocument {
///     title: &str, // 错误：字段是引用，必须在结构体上声明生命周期
/// }
/// ```
///
/// 对照 `Document<'a>` 的写法：结构体名后加 `<'a>`，字段写成 `&'a str`。
pub struct _Placeholder;

/// 演示：**多个字段可以拥有不同生命周期**，也可以共享同一个。
///
/// 下面的结构体让 `title` 与 `body` 各自独立存活（`'a` 与 `'b`）。
/// 相比 `Document<'a>`（两者共享一个 `'a`，必须同时有效），
/// `SplitDocument` 更宽松：title 和 body 可以来自不同作用域的数据。
///
/// ```
/// use lifetime_showcase::struct_lifetime::SplitDocument;
///
/// let title = String::from("短命标题");
/// let r;
/// {
///     let body = String::from("正文可以更长");
///     let d = SplitDocument::new(&title, &body);
///     r = d.title.to_string(); // 只借用 title 即可
/// }
/// assert_eq!(r, "短命标题");
/// ```
pub struct SplitDocument<'a, 'b> {
    pub title: &'a str,
    pub body: &'b str,
}

impl<'a, 'b> SplitDocument<'a, 'b> {
    pub fn new(title: &'a str, body: &'b str) -> Self {
        SplitDocument { title, body }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_borrows_until_source_drops() {
        let title = String::from("t");
        let body = "b".repeat(100);
        let doc = Document::new(&title, &body);
        assert_eq!(doc.title, "t");
        assert!(doc.summary().contains("《t》"));
    }

    #[test]
    fn document_display_works() {
        let title = String::from("T");
        let body = String::from("B");
        let doc = Document::new(&title, &body);
        assert_eq!(doc.to_string(), "# T\n\nB");
    }

    #[test]
    fn split_document_allows_independent_lifetimes() {
        let title = String::from("T");
        let owned;
        {
            let body = String::from("B");
            let d = SplitDocument::new(&title, &body);
            // body 先于 title 结束也没关系，因为我们只用了 title
            owned = d.title.to_string();
        }
        assert_eq!(owned, "T");
    }
}
