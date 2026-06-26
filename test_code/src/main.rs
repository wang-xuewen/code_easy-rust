// 1. 函数生命周期标注（显式）
// 这里不符合省略规则，所以需要显示标注生命周期
// 函数作用是比较两个传入参数，哪个长度长就返回哪个
// 两个传入参数和返回值都是字符串引用类型，所以需要标注生命周期
// 而且参数和返回值都要满足生命周期 'a 这么大的生命周期
// 这里不是约定'a多大，而是约定了返回值肯定有效，因为只要x y有效，返回值肯定有效。（一样的生命周期决定了返回值和x y活的至少一样久）
// 如果编译器发现前后代码不符合这个要求，就会报错
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

// 2. 结构体生命周期的例子
// 成员变量是一个字符串引用类型，他的值一定是引用外部字符串的，所以需要标注生命周期，用以告诉编译器，结构体如果存活'a这么长时间，那么这个被已用的外部字符串的存活时间必须大于等于'a
// 编译时如果代码不满足这个要求，编译器就会报错
// 如果不标注，这里又不符合生命周期省略规则，编译器不能自动推断出生命周期，此时会拒绝编译，并要求你手动标注。
struct Excerpt<'a> {
    part: &'a str,
}

// impl<'a> 中的 <'a> 告诉编译器：我要在这个代码块中声明一个名为 'a 的生命周期泛型参数，以便在后续的类型和方法中使用它
// Excerpt<'a> 中的 <'a> 告诉编译器：“我要把上面刚刚声明的那个生命周期 'a，传递给 Excerpt。”
impl<'a> Excerpt<'a> {
    // 3. 方法中生命周期省略
    // 满足生命周期省略规则三，所以不用手动写生命周期
    // 编译器编译时会自己补足
    fn first_sentence(&self) -> &str {
        // Rust 中非常经典且优雅的链式调用（Method Chaining）。它的作用是从字符串中提取第一个句子（即第一个 . 之前的内容）
        // self.part.split('.') 将字符串 self.part 按照 . 进行切割
        // .next() 让迭代器向前迈一步，获取下一个元素
        // .unwrap_or("") 如果迭代器没有下一个元素，则返回一个空字符串
        self.part.split('.').next().unwrap_or("")
    }

    // 是因为'a 在 impl<'a> 处已经声明过了，所以这里不需要再重新声明

    // 4. 方法显式标注（多个不同生命周期）
    // cut<'b, 'c> 中 <'b, 'c> 是声明了两个生命周期参数 'b 和 'c，后续我将使用这两个生命周期参数，'a 在 impl<'a> 处已经声明过了，所以这里不需要再重新声明
    // &'a self 表示第一个参数是结构体自身的引用，引用的生命周期为 'a
    // other: &'b str 表示第二个参数是字符串引用，引用的生命周期为 'b。
    // 返回值生命周期为 'c，表示返回的字符串切片，与 suffix 的生命周期一致
    fn cut<'b, 'c>(&'a self, other: &'b str, suffix: &'c str) -> &'c str {
        // 简单演示：如果 suffix 非空，返回 suffix 的前3个字符；否则返回 other 的前3个字符
        // 注意：因为返回类型标注为 &'c str，所以这里必须返回与 suffix 生命周期相关的数据
        if other.len() > 5 {
            // 取 3 和 suffix.len() 中的较小值，赋值给变量 end
            let end = std::cmp::min(3, suffix.len());

            // suffix[0..end] 表示从 0 开始，取到 end 位置的子字符串
            &suffix[0..end]
        } else {
            suffix
        }
    }
}

// 5. 静态生命周期（全局存活）
// 'static：这是一个特殊且内置的生命周期。它表示这个引用指向的数据，其生命周期将贯穿整个程序的运行期。
const GREETING: &'static str = "Hello, world!";

// 6. 生命周期省略规则（输入/输出自动推断）
// 满足生命周期省略规则二，不用手动写生命周期，编译器编译时会自动推导补足
// 函数的作用是从字符串中提取第一个单词(把字符串按照空格分割，返回第一个单词)
fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

fn main() {
    // 创建了一个 String 类型变量 string1
    // 注意：String类型存储在堆上
    let string1 = String::from("long");

    // 创建了一个字符串字面量
    // 字符串字面量: 直接硬编码到程序可执行文件中的数据段（常量区）中，在程序运行期间不可变。
    // 所以这句代码的意思相当于在栈上创建了一个只读的字符串引用(&str)，它指向程序二进制文件中硬编码的、永远有效的一个字符串"short"。
    let string2 = "short";

    let result = longest(&string1, string2);
    println!("Longest: {}", result); // 输出: long

    // 8. 结构体使用
    // 初始化字符串novel
    let novel = String::from("Call me Ishmael. Some years ago...");
    // 创建一个 Excerpt 结构体实例
    let excerpt = Excerpt { part: &novel[..] };
    println!("First: {}", excerpt.first_sentence());

    {
        // --- 演示多个生命周期参数的 cut 函数 ---
        let suffix = String::from("SuffixData"); // 'c 生命周期开始

        {
            let other = String::from("OtherData"); // 'b 生命周期开始

            // 调用 cut，传入 &other ('b) 和 &suffix ('c)
            // 因为 other.len() > 5，所以会返回 suffix 的前3个字符
            let result3 = excerpt.cut(&other, &suffix);
            println!("Cut result: {}", result3); // 输出: Suf
        } // 'b (other) 在这里销毁

        // 因为 result3 的生命周期被约束为 'c (suffix)，
        // 所以即使 other 已经销毁，result3 依然可以安全使用
        println!("Suffix still valid: {}", suffix);

        // --- 演示 other 长度 <= 5 的情况 ---
        let short_other = "Hi"; // 长度为 2 (<=5)
        let result4 = excerpt.cut(short_other, &suffix);
        println!("Cut result 4: {}", result4); // 输出: SuffixData (返回整个 suffix)
    } // 'c (suffix) 在这里销毁
}
