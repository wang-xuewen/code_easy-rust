## Rust 动态分发（Dynamic Dispatch）详解

动态分发是 Rust 中实现多态性的两种方式之一（另一种是静态分发），它允许在**运行时**确定具体调用哪个方法实现。

---

### 核心概念

| 特性 | 静态分发（泛型） | 动态分发（Trait 对象） |
|------|------------------|------------------------|
| 绑定时机 | 编译时 | 运行时 |
| 性能 | 零成本抽象，可内联 | 有虚函数调用开销 |
| 语法 | `impl Trait` 或 `<T: Trait>` | `&dyn Trait` 或 `Box<dyn Trait>` |
| 内存布局 | 单态化（每个类型生成单独代码） | 胖指针（数据指针 + vtable 指针） |

---

### 基本用法

#### 1. Trait 对象声明

```rust
trait Animal {
    fn speak(&self);
}

struct Dog;
struct Cat;

impl Animal for Dog {
    fn speak(&self) { println!("Woof!"); }
}

impl Animal for Cat {
    fn speak(&self) { println!("Meow!"); }
}

// 动态分发：使用 &dyn 或 Box<dyn>
fn make_sound(animal: &dyn Animal) {
    animal.speak();
}

fn main() {
    let dog = Dog;
    let cat = Cat;
    
    make_sound(&dog);  // 编译时不知道具体类型
    make_sound(&cat);  // 运行时通过 vtable 查找方法
}
```

#### 2. 集合中存储不同类型

```rust
fn main() {
    let animals: Vec<Box<dyn Animal>> = vec![
        Box::new(Dog),
        Box::new(Cat),
    ];
    
    for animal in &animals {
        animal.speak();  // 动态分发
    }
}
```

---

### 底层原理：胖指针（Fat Pointer）

动态分发通过**胖指针**实现，每个 Trait 对象包含：

```rust
// &dyn Animal 的内存布局
struct FatPointer {
    data: *mut (),        // 指向实际数据的指针
    vtable: *mut VTable,  // 指向虚函数表的指针
}

struct VTable {
    size: usize,          // 对象大小
    align: usize,         // 对齐方式
    drop: fn(*mut ()),    // 析构函数
    speak: fn(*mut ()),   // 方法指针
}
```

---

### 对象安全（Object Safety）

并非所有 Trait 都能转为 Trait 对象，必须满足**对象安全**：

```rust
// ✅ 对象安全
trait Safe {
    fn method(&self);                    // 接收 self 引用
    fn method2(&self, x: i32);           // 可以有其他参数
}

// ❌ 非对象安全（不能创建 dyn Safe）
trait Unsafe {
    fn generic<T>(&self, t: T);          // 泛型方法
    fn static_method() -> Self;          // 关联函数返回 Self
    fn consume(self);                   // 按值消费 self
}

// 编译错误
// let obj: &dyn Unsafe = ...; // ❌
```

**对象安全的必要条件**：
1. 所有方法必须是对象安全的
2. 方法不能有泛型参数
3. 方法不能返回 `Self`（除非 `Self: Sized`）
4. 方法不能按值接收 `self`（即 `self` 必须为引用）

---

### 动态分发 vs 静态分发

```rust
// 静态分发（泛型）
fn static_dispatch<T: Animal>(animal: &T) {
    animal.speak();  // 编译时确定具体类型
}

// 动态分发（Trait 对象）
fn dynamic_dispatch(animal: &dyn Animal) {
    animal.speak();  // 运行时通过 vtable 查找
}

// 调用区别
let dog = Dog;
static_dispatch(&dog);   // 单态化生成 Dog 版本
dynamic_dispatch(&dog);  // 使用 vtable 动态调用
```

| 对比维度 | 静态分发 | 动态分发 |
|---------|---------|---------|
| 代码体积 | 较大（N 个类型生成 N 份代码） | 较小（一份通用代码） |
| 调用速度 | 快（可直接内联） | 慢（间接调用，无法内联） |
| 编译时间 | 较长 | 较短 |
| 灵活性 | 编译时确定 | 运行时动态 |
| 类型擦除 | 不支持 | 支持 |

---

### 常见应用场景

#### 1. 插件系统/策略模式

```rust
trait Encryption {
    fn encrypt(&self, data: &[u8]) -> Vec<u8>;
}

struct AES;
impl Encryption for AES { /* ... */ }

struct ChaCha20;
impl Encryption for ChaCha20 { /* ... */ }

// 运行时选择加密算法
fn encrypt_data(enc: &dyn Encryption, data: &[u8]) -> Vec<u8> {
    enc.encrypt(data)
}
```

#### 2. 回调/闭包类型擦除

```rust
// 存储不同类型的闭包
type Callback = Box<dyn Fn(i32) -> i32>;

struct Processor {
    callbacks: Vec<Callback>,
}

impl Processor {
    fn add_callback<F: Fn(i32) -> i32 + 'static>(&mut self, f: F) {
        self.callbacks.push(Box::new(f));
    }
}
```

#### 3. 避免泛型爆炸

```rust
// 复杂数据结构中，避免泛型参数传播
struct Game {
    objects: Vec<Box<dyn GameObject>>,  // 存储不同类型的对象
}

trait GameObject {
    fn update(&mut self);
    fn draw(&self);
}
```

---

### 性能考虑与优化

```rust
// 1. 优先使用 &dyn 而非 Box<dyn>（减少堆分配）
fn process(animal: &dyn Animal) { /* ... */ }

// 2. 使用 dyn* 优化（Rust 实验性特性，减少间接层）
// #![feature(dyn_star)]
// fn process(animal: dyn* Animal) { /* ... */ }

// 3. 考虑使用 enum 替代（如果类型集合有限）
enum AnimalEnum {
    Dog(Dog),
    Cat(Cat),
}
// 性能更好但灵活性较差
```

---

### 常见陷阱与注意事项

1. **生命周期标注**
```rust
struct Holder<'a> {
    animal: &'a dyn Animal,  // 必须标注生命周期
}
```

2. **Trait 对象的 `Sized` 约束**
```rust
// dyn Trait 默认是 !Sized
fn takes_dyn<T: ?Sized + Animal>(animal: &T) { /* 可以接受动态分发 */ }
```

3. **不能直接使用 Trait 对象的方法**
```rust
// ❌ 错误：trait 对象不能调用 Self 类型关联函数
trait Builder {
    fn new() -> Self;  // 非对象安全
}
```

4. **跨 crate 的 Trait 对象**
```rust
// 需要在定义处标记为 dyn 兼容（对象安全）
pub trait PublicTrait {
    fn method(&self);
    // 不能有泛型或返回 Self
}
```

---

### 总结

- **选择动态分发**：需要类型擦除、在集合中存储不同类型、运行时才能确定类型
- **选择静态分发**：追求性能、类型数量有限且编译时已知
- **核心权衡**：灵活性与性能的交换

动态分发是 Rust 实现面向对象风格多态的核心机制，正确使用可以大幅提升代码的灵活性和可扩展性。