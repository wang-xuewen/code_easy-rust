//! 方法中的生命周期：`impl<'a>`、`&self`、`&mut self`。
//!
//! 方法签名涉及生命周期时，有三个层次需要区分清楚：
//!
//! 1. **`impl<'a>`**：声明该 impl 块针对的生命周期参数，即「结构体的生命周期」。
//! 2. **`&self` / `&mut self`**：方法接收者的借用，本身也有独立的生命周期
//!    （省略规则 3 会让返回引用默认绑定到它）。
//! 3. **返回类型**：返回 `&'a str`（绑定结构体数据）还是 `&str`（绑定 `self` 借用），
//!    语义截然不同。

/// 一个文本缓冲区，展示方法返回值生命周期的两种绑定方式。
pub struct Buffer<'a> {
    content: &'a str,
}

impl<'a> Buffer<'a> {
    pub fn new(content: &'a str) -> Self {
        Buffer { content }
    }

    /// 返回 `&str`：按省略规则 3，生命周期绑定到 `&self`。
    ///
    /// 含义：只要这个 `&self` 借用还有效，返回值就有效。
    /// 多数只读方法用这种写法即可，简洁且安全。
    pub fn as_str(&self) -> &str {
        self.content
    }

    /// 返回 `&'a str`：显式绑定到**结构体的生命周期**，而非 `&self` 借用。
    ///
    /// 二者区别在 `&mut self` 方法里最明显：即便方法接收的是可变借用，
    /// 也仍然可以「透过结构体」返回指向其内部数据的共享引用 `&'a str`。
    /// 注意这里接收者仍是 `&self`（只读），下一条 `as_str_long` 才是关键对比。
    pub fn as_str_typed(&self) -> &'a str {
        self.content
    }

    /// 对比实验：把结构体生命周期 `'a` 与「方法接收者借用」分离。
    ///
    /// 下面这个函数签名含义是——返回值的存活只取决于入参 `other` 的
    /// 生命周期 `'b`，与 `self` 的借用时长无关。这在你需要返回一个
    /// 与调用对象本身无关的引用时很有用。
    pub fn or_longest<'b>(&self, other: &'b str) -> &'b str
    where
        'a: 'b, // 约束：结构体数据至少活得比 'b 久（见 bounds 模块）
    {
        if self.content.len() >= other.len() {
            // 由于 'a: 'b，结构体的 'a 可以被「缩短」成 'b 返回
            self.content
        } else {
            other
        }
    }
}

/// 演示：方法签名上的生命周期省略是如何被补全的。
///
/// 你写：
/// ```text
/// fn first_line(&self) -> &str
/// ```
/// 编译器按规则 3 补全为：
/// ```text
/// fn first_line<'s>(&'s self) -> &'s str
/// ```
/// 其中 `'s` 是 `self` 这个借用的生命周期，通常**短于**结构体的 `'a`。
///
/// ```
/// use lifetime_showcase::methods::Buffer;
///
/// let data = String::from("alpha\nbeta");
/// let buf = Buffer::new(&data);
/// let line = buf.as_str();
/// assert!(line.starts_with("alpha"));
/// ```
pub fn _doc_demo() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_binds_to_self_borrow() {
        let data = String::from("hello");
        let buf = Buffer::new(&data);
        let s = buf.as_str();
        assert_eq!(s, "hello");
        // 返回的是借用，data 仍拥有数据
        assert_eq!(data, "hello");
    }

    #[test]
    fn or_longest_picks_longer() {
        let data = String::from("short");
        let buf = Buffer::new(&data);
        let other = String::from("a-much-longer-string");
        // 返回 other 的引用
        assert_eq!(buf.or_longest(&other), "a-much-longer-string");

        let data2 = String::from("this-is-long");
        let buf2 = Buffer::new(&data2);
        let other2 = "short";
        // 返回 self 的内容（因 'a: 'b 成立）
        assert_eq!(buf2.or_longest(other2), "this-is-long");
    }

    #[test]
    fn typed_ref_still_borrowed() {
        let data = String::from("abc");
        let buf = Buffer::new(&data);
        let r: &str = buf.as_str_typed();
        assert_eq!(r, "abc");
    }
}
