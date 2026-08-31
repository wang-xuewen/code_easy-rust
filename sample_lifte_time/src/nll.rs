//! 非词法作用域生命周期（Non-Lexical Lifetimes，NLL）。
//!
//! 早期 Rust 的借用检查基于**词法作用域**：一个变量的借用会持续到它所在
//! 代码块结束。这导致很多「实际上已经用完引用、之后才发生冲突」的合法代码
//! 被误判为错误。
//!
//! NLL（Rust 2018 起默认启用）改变了规则：借用结束于该引用的**最后一次使用**，
//! 而非代码块末尾。这让大量原本需要绕行的写法变得自然直接。
//!
//! 本模块展示的是「在 NLL 下合法、在旧版词法生命周期下会被拒绝」的典型代码。

/// 演示：不可变借用结束后，可以立刻进行可变操作。
///
/// 在词法生命周期下，`first` 的借用被认为持续到函数结尾，`data.push` 会报错；
/// NLL 下，`first` 在 `*first` 读取完后即结束，`push` 完全合法。
///
/// ```
/// use lifetime_showcase::nll::read_then_mutate;
/// assert_eq!(read_then_mutate(), 1);
/// ```
pub fn read_then_mutate() -> i32 {
    let mut data = vec![1, 2, 3];
    let first = &data[0]; // 不可变借用开始
    let n = *first; // first 最后一次使用，借用在此结束
    data.push(4); // NLL 下合法：first 已不再被使用
    n
}

/// 演示：同一作用域内「先不可变借用、再可变借用」在 NLL 下无需额外花括号。
///
/// ```
/// use lifetime_showcase::nll::print_then_mutate;
/// assert_eq!(print_then_mutate(), 11);
/// ```
pub fn print_then_mutate() -> i32 {
    let mut x = 10;
    let r1 = &x;
    println!("r1 = {}", r1); // r1 最后一次使用
    let r2 = &mut x; // NLL 下合法：r1 已结束
    *r2 += 1;
    x
}

/// 演示：可变借用结束后，仍可恢复不可变借用。
///
/// ```
/// use lifetime_showcase::nll::mutate_then_read;
/// assert_eq!(mutate_then_read(), 11);
/// ```
pub fn mutate_then_read() -> i32 {
    let mut x = 10;
    let r = &mut x;
    *r += 1; // r 最后一次使用
    let value = x; // NLL 下合法：r 已结束
    value
}

/// 一个接近生产场景的综合例子：**处理完借用数据后，再安全地取得所有权**。
///
/// 展示 NLL 如何让「借用 → 使用 → 释放借用 → 移动所有权」的流程无需人工拆分作用域。
///
/// ```
/// use lifetime_showcase::nll::process_then_consume;
/// let s = String::from("rustacean");
/// let (len, owned) = process_then_consume(s);
/// assert_eq!(len, 9);
/// assert_eq!(owned, "RUSTACEAN");
/// ```
pub fn process_then_consume(s: String) -> (usize, String) {
    // 阶段一：借用读取
    let len = s.len();
    let upper = s.to_uppercase();
    // 阶段二：s 的借用已全部结束，可以移动所有权（无需 clone）
    (len, upper)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_then_mutate_works() {
        assert_eq!(read_then_mutate(), 1);
    }

    #[test]
    fn print_then_mutate_works() {
        assert_eq!(print_then_mutate(), 11);
    }

    #[test]
    fn mutate_then_read_works() {
        assert_eq!(mutate_then_read(), 11);
    }

    #[test]
    fn process_then_consume_works() {
        let s = String::from("abc");
        let (len, owned) = process_then_consume(s);
        assert_eq!(len, 3);
        assert_eq!(owned, "ABC");
    }
}
