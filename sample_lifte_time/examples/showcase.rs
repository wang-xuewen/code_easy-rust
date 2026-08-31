//! 综合演示：把本项目所有生命周期概念串起来跑一遍。
//!
//! 运行方式：
//! ```bash
//! cargo run --example showcase
//! ```

use lifetime_showcase::{
    basics, bounds, elision, methods, multiple_lifetimes, nll, static_lifetime,
    struct_lifetime, trait_object, variance,
};

fn main() {
    println!("===== 1. 生命周期基础：引用、借用 =====");
    let a = String::from("hello");
    let b = "world!";
    println!("longest(&a, b) = {}", basics::longest(&a, b));
    println!("greet() = {}", basics::greet());

    println!("\n===== 2. 生命周期省略规则 =====");
    println!("first_word(\"hello world\") = {}", elision::first_word("hello world"));
    let reader = elision::Reader::new("one\ntwo\nthree");
    println!("reader.line(1) = {}", reader.line(1));

    println!("\n===== 3. 结构体中的生命周期 =====");
    let title = String::from("生命周期入门");
    let body = String::from("# 引用\n每个引用都有生命周期");
    let doc = struct_lifetime::Document::new(&title, &body);
    println!("doc.summary() = {}", doc.summary());

    println!("\n===== 4. 方法中的生命周期 =====");
    let data = String::from("alpha\nbeta");
    let buf = methods::Buffer::new(&data);
    println!("buf.as_str() = {}", buf.as_str());
    println!("buf.or_longest(\"longer\") = {}", buf.or_longest("longer"));

    println!("\n===== 5. 多个生命周期 =====");
    let x = String::from("AAA");
    let y = String::from("BBB");
    println!("first(&x, &y) = {}", multiple_lifetimes::first(&x, &y));
    let log = String::from("ERROR: boom\nINFO: ok\nERROR: again");
    let kw = String::from("ERROR");
    let hits: Vec<&str> = multiple_lifetimes::LogIter::new(&log, &kw).collect();
    println!("LogIter 命中: {hits:?}");

    println!("\n===== 6. 'static 生命周期 =====");
    println!("literal() = {}", static_lifetime::literal());
    println!("leak_to_static() = {}", static_lifetime::leak_to_static());

    println!("\n===== 7. 生命周期约束 T: 'a =====");
    let key = String::from("k");
    let cache = bounds::Cache::new(&key, 42usize);
    println!("cache.value() = {}", cache.value());

    println!("\n===== 8. trait 对象生命周期 =====");
    let owned = trait_object::make_static_renderer();
    let source = String::from("borrowed");
    let borrowed = trait_object::make_borrowed_renderer(&source);
    println!("owned.render() = {}", owned.render());
    println!("borrowed.render() = {}", borrowed.render());

    println!("\n===== 9. 变体（协变 / 逆变 / 不变） =====");
    variance::accepts_any_lifetime(variance::covariant_demo());
    variance::contravariance_demo();
    variance::invariance_demo();
    println!("协变 / 逆变 / 不变 演示均编译并运行成功");

    println!("\n===== 10. 非词法作用域生命周期 NLL =====");
    println!("read_then_mutate() = {}", nll::read_then_mutate());
    println!("print_then_mutate() = {}", nll::print_then_mutate());
    println!("mutate_then_read() = {}", nll::mutate_then_read());

    println!("\n全部 10 个主题演示完成 ✅");
}
