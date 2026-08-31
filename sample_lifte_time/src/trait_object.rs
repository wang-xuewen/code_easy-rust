//! trait 对象的生命周期。
//!
//! trait 对象（`dyn Trait`）也有生命周期，只是通常被省略。关键规则：
//!
//! > **trait 对象的默认生命周期是 `'static`。**
//!
//! 因此：
//! - `Box<dyn Trait>` 实际是 `Box<dyn Trait + 'static>`；
//! - `&dyn Trait` 实际是 `&'a (dyn Trait + 'a)`（引用自身的生命周期被默认带入）。
//!
//! 当你需要一个「持有短命借用」的 trait 对象时，必须显式写 `+ 'a`。

use std::fmt;

/// 一个极简的「格式化输出」trait，用于演示 trait 对象。
pub trait Render {
    fn render(&self) -> String;
}

/// 拥有所有权的实现：不含任何借用，天然满足 `'static`。
pub struct OwnedDoc {
    pub text: String,
}

impl Render for OwnedDoc {
    fn render(&self) -> String {
        self.text.clone()
    }
}

/// 借用数据的实现：内部持有 `&'a str`，因此它的 trait 对象必须标注 `+ 'a`。
pub struct BorrowedDoc<'a> {
    pub text: &'a str,
}

impl Render for BorrowedDoc<'_> {
    fn render(&self) -> String {
        self.text.to_string()
    }
}

/// 返回一个 `'static` trait 对象：因为 `OwnedDoc` 拥有数据，可以一直存活。
///
/// ```
/// use lifetime_showcase::trait_object::make_static_renderer;
/// let r = make_static_renderer();
/// assert_eq!(r.render(), "owned");
/// ```
pub fn make_static_renderer() -> Box<dyn Render> {
    // 等价于 Box<dyn Render + 'static>
    Box::new(OwnedDoc {
        text: "owned".to_string(),
    })
}

/// 返回一个**借用数据**的 trait 对象：必须把生命周期 `'a` 显式写进类型。
///
/// 注意签名里的 `+ 'a`：它告诉编译器，这个 `Box<dyn Render>` 内部可能持有
/// 生命周期为 `'a` 的引用，因此整个对象不能比 `'a` 活得更久。
///
/// ```
/// use lifetime_showcase::trait_object::make_borrowed_renderer;
/// let source = String::from("borrowed");
/// let r = make_borrowed_renderer(&source);
/// assert_eq!(r.render(), "borrowed");
/// ```
pub fn make_borrowed_renderer<'a>(text: &'a str) -> Box<dyn Render + 'a> {
    Box::new(BorrowedDoc { text })
}

/// 演示错误：**没有标注 `+ 'a` 时，借用数据的 trait 对象无法编译**。
///
/// ```compile_fail
/// // 错误：BorrowedDoc<'a> 含借用，不能放进默认 'static 的 Box<dyn Render>
/// fn broken<'a>(text: &'a str) -> Box<dyn Render> {
///     Box::new(BorrowedDoc { text })
/// }
/// ```
pub struct _Doc;

/// 用 `Box<dyn Render + 'a>` 承载**异构**的渲染器集合——trait 对象的实际价值。
///
/// 所有渲染器只要生命周期满足 `'a`，就能放进同一个 `Vec`，实现运行时多态。
///
/// ```
/// use lifetime_showcase::trait_object::{pipeline, BorrowedDoc, OwnedDoc, Render};
///
/// let source = String::from("from-borrow");
/// let renderers: Vec<Box<dyn Render + '_>> = vec![
///     Box::new(OwnedDoc { text: "from-owned".into() }),
///     Box::new(BorrowedDoc { text: &source }),
/// ];
/// assert_eq!(pipeline(&renderers), "from-owned | from-borrow");
/// ```
pub fn pipeline(renderers: &[Box<dyn Render + '_>]) -> String {
    renderers
        .iter()
        .map(|r| r.render())
        .collect::<Vec<_>>()
        .join(" | ")
}

impl fmt::Debug for dyn Render + '_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Render({})", self.render())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_renderer_works() {
        let r = make_static_renderer();
        assert_eq!(r.render(), "owned");
    }

    #[test]
    fn borrowed_renderer_tracks_source() {
        let source = String::from("borrowed");
        let r = make_borrowed_renderer(&source);
        assert_eq!(r.render(), "borrowed");
    }

    #[test]
    fn pipeline_joins_heterogeneous_renderers() {
        let source = String::from("B");
        let renderers: Vec<Box<dyn Render + '_>> = vec![
            Box::new(OwnedDoc {
                text: "A".to_string(),
            }),
            Box::new(BorrowedDoc { text: &source }),
        ];
        assert_eq!(pipeline(&renderers), "A | B");
    }
}
