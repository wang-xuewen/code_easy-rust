use my_cargo_demo::Data;

fn main() {
    println!("===== Cargo.toml 演示 =====");
    println!("包名: {}", env!("CARGO_PKG_NAME"));
    println!("版本: {}", env!("CARGO_PKG_VERSION"));
    println!("作者: {}", env!("CARGO_PKG_AUTHORS"));
    println!("描述: {}", env!("CARGO_PKG_DESCRIPTION"));
    println!();
    
    // 基础功能演示
    let data = Data::new("演示数据", 100);
    println!("数据对象: {:?}", data);
    println!("JSON格式: {}", data.to_json());
    println!();
    
    // 条件编译演示
    println!("启用的特性:");
    println!("  basic: {}", cfg!(feature = "basic"));
    println!("  regex: {}", cfg!(feature = "regex"));
    println!("  rand: {}", cfg!(feature = "rand"));
    println!();
    
    // 可选功能演示
    #[cfg(feature = "regex")]
    {
        println!("=== Regex 功能 ===");
        let result = my_cargo_demo::validate_pattern(r"^\d+$", "12345");
        println!("数字匹配: {}", result);
    }
    
    #[cfg(feature = "rand")]
    {
        println!("=== Random 功能 ===");
        let num = my_cargo_demo::random_number(100);
        println!("随机数: {}", num);
    }
    
    #[cfg(all(not(feature = "regex"), not(feature = "rand")))]
    {
        println!("提示: 启用更多特性以获得额外功能");
        println!("试试: cargo run --features full");
    }
}