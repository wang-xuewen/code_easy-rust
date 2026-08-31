//! 生命周期变体（Variance）：协变、逆变、不变。
//!
//! 这是生命周期里最进阶、也最容易被忽视的主题。它描述的是：
//! **当两个生命周期存在「长短」关系时，包含它们的类型之间是否也存在子类型关系？**
//!
//! ## 前置：子类型
//!
//! 若 `'long: 'short`（`'long` 比 `'short` 活得久），则
//! `&'long T` 是 `&'short T` 的**子类型**——可以在任何需要 `&'short T`
//! 的位置使用 `&'long T`（一个长命引用可以安全地当作短命引用用）。
//!
//! ## 三种变体
//!
//! | 变体 | 含义 | 典型类型 |
//! |---|---|---|
//! | 协变 covariant | 子类型关系**保持** | `&'a T`（在 `'a` 与 `T` 上） |
//! | 逆变 contravariant | 子类型关系**反转** | 函数指针的参数位 `fn(&'a T)` |
//! | 不变 invariant | **无**子类型关系 | `&'a mut T`（在 `T` 上）、`Cell<T>` |
//!
//! Rust 编译器会自动推导每个类型在各位置上的变体，绝大多数时候你无需关心，
//! 但理解它能帮你读懂很多「看起来合理却编译不过」的报错。

/// 协变（covariant）：`&'a T` 在生命周期 `'a` 上是协变的。
///
/// 一个 `&'static str` 可以传给任何要求 `&'a str` 的函数，因为 `'static`
/// 比任意 `'a` 都长。子类型关系被**原样保持**——这就是协变。
///
/// ```
/// use lifetime_showcase::variance::{covariant_demo, accepts_any_lifetime};
///
/// let s: &'static str = "long-lived";
/// accepts_any_lifetime(s); // &'static str 协变为 &'a str
/// assert_eq!(covariant_demo(), "still-long");
/// ```
// 此处 'a 刻意显式写出，用于强调「任意生命周期」的语义。
#[allow(clippy::needless_lifetimes)]
pub fn accepts_any_lifetime<'a>(_r: &'a str) {}

/// 返回 `&'static str`，并在调用点演示它可被「缩短」使用。
pub fn covariant_demo() -> &'static str {
    "still-long"
}

/// 逆变（contravariant）：函数**参数**位置的关系是反转的。
///
/// 直觉：一个「能处理**短命**引用」的函数（更通用），必然也能处理
/// 「长命」引用，因此可以被当作「只处理长命引用」的函数来用。
/// 这就是关系反转——逆变。
///
/// ```
/// use lifetime_showcase::variance::contravariance_demo;
/// contravariance_demo();
/// ```
pub fn contravariance_demo() {
    // 接受任意生命周期引用的通用处理函数（'a 刻意显式写出）
    #[allow(clippy::needless_lifetimes)]
    fn generic<'a>(_r: &'a str) {}

    // 逆变：generic（接受任意 &str）可以赋给只要求 &'static str 的函数指针。
    // 换个角度：需要「能处理任意引用」的位置，反而不接受「只处理 'static」的函数，
    // 因为调用方可能传入短命引用——这正是参数位关系反转的体现。
    let _f: fn(&'static str) = generic;
}

/// 不变（invariant）：`&mut T` 在 `T` 上不变，防止协变导致悬垂。
///
/// 为什么 `&mut &'static str` 不能当 `&mut &'a str` 用？假如允许（协变），
/// 就能通过一个「短命引用槽」把短命引用写进原本要求 `'static` 的位置，产生悬垂。
/// 因此 `&mut T` 在 `T` 上被设计为**不变**：
///
/// ```compile_fail
/// fn demo() {
///     let mut slot: &'static str = "long";
///     {
///         let short = String::from("short");
///         // 错误：&mut &'static str 不能协变成 &mut &str（不变），阻止了悬垂
///         let slot_as_any: &mut &str = &mut slot;
///         *slot_as_any = &short;
///     }
///     // 若上面合法，这里 slot 将指向已释放的 short —— 灾难
/// }
/// ```
pub fn invariance_demo() {
    // 可运行对照：当类型精确匹配时，&mut 借用完全没问题
    let mut slot: &'static str = "long";
    let same: &mut &'static str = &mut slot;
    *same = "another-static";
    assert_eq!(slot, "another-static");
}

/// 用 `Cell<T>` 佐证不变性：内部可变性容器在 `T` 上也是不变的。
///
/// 原因与 `&mut T` 相同——允许写入意味着允许「把短命引用塞进长命槽位」，
/// 所以 `Cell<&'static str>` 同样不能协变成 `Cell<&str>`。
pub struct _CellDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covariant_shortens_static() {
        let s: &'static str = "hello";
        accepts_any_lifetime(s);
        assert_eq!(covariant_demo(), "still-long");
    }

    #[test]
    fn contravariant_function_pointer_assigns() {
        // 仅仅调用，确认编译与运行无 panic
        contravariance_demo();
    }

    #[test]
    fn invariant_mut_allows_exact_match() {
        invariance_demo();
    }
}
