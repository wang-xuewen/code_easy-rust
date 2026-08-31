//! 集成测试：从「外部使用者」视角验证公共 API 的语义正确性。
//!
//! 单元测试（各模块内的 `#[cfg(test)] mod tests`）覆盖实现细节，
//! 这里的集成测试则验证：库对外暴露的生命周期契约是否真的安全、可用。

use lifetime_showcase::{
    basics, bounds, elision, methods, multiple_lifetimes, nll, static_lifetime,
    struct_lifetime, trait_object, variance,
};

#[test]
fn basics_longest_returns_borrowed_data() {
    let short = String::from("ab");
    let long = String::from("abcdefgh");
    let r = basics::longest(&short, &long);
    // r 借用 long，long 仍拥有数据
    assert_eq!(r, "abcdefgh");
    assert_eq!(long, "abcdefgh");
}

#[test]
fn elision_reader_never_copies_source() {
    let text = String::from("alpha\nbeta\ngamma");
    let reader = elision::Reader::new(&text);
    // 返回的是对 text 的借用，而不是新字符串
    assert_eq!(reader.line(0), "alpha");
    assert_eq!(reader.line(2), "gamma");
}

#[test]
fn struct_document_lifetime_tied_to_source() {
    let title = String::from("T");
    let body = String::from("B");
    let doc = struct_lifetime::Document::new(&title, &body);
    // 借用对象只能活到 source 存活期间，这里在同一作用域内安全使用
    assert!(doc.summary().starts_with("《T》"));
}

#[test]
fn methods_buffer_or_longest_chooses_correctly() {
    let data = String::from("short");
    let buf = methods::Buffer::new(&data);
    assert_eq!(buf.or_longest("a-longer-value"), "a-longer-value");

    let data2 = String::from("a-much-longer-value");
    let buf2 = methods::Buffer::new(&data2);
    assert_eq!(buf2.or_longest("s"), "a-much-longer-value");
}

#[test]
fn multiple_lifetimes_log_iter_filters() {
    let text = String::from("ERR a\nOK b\nERR c");
    let kw = String::from("ERR");
    let got: Vec<&str> = multiple_lifetimes::LogIter::new(&text, &kw).collect();
    assert_eq!(got, vec!["ERR a", "ERR c"]);
}

#[test]
fn static_lifetime_owned_data_satisfies_bound() {
    static_lifetime::spawn_safe(String::from("owned"));
    assert_eq!(static_lifetime::literal(), "I live as long as the program");
}

#[test]
fn bounds_cache_preserves_value() {
    let key = String::from("config.key");
    let cache = bounds::Cache::new(&key, String::from("config.value"));
    assert_eq!(cache.value(), "config.value");
}

#[test]
fn trait_object_heterogeneous_pipeline() {
    let source = String::from("borrowed");
    let renderers: Vec<Box<dyn trait_object::Render + '_>> = vec![
        Box::new(trait_object::OwnedDoc {
            text: "owned".into(),
        }),
        Box::new(trait_object::BorrowedDoc { text: &source }),
    ];
    assert_eq!(trait_object::pipeline(&renderers), "owned | borrowed");
}

#[test]
fn variance_rules_hold_at_api_level() {
    variance::accepts_any_lifetime("literal");
    variance::contravariance_demo();
    variance::invariance_demo();
}

#[test]
fn nll_flows_are_all_safe() {
    assert_eq!(nll::read_then_mutate(), 1);
    assert_eq!(nll::print_then_mutate(), 11);
    assert_eq!(nll::mutate_then_read(), 11);
    let s = String::from("xyz");
    let (len, owned) = nll::process_then_consume(s);
    assert_eq!((len, owned.as_str()), (3, "XYZ"));
}
