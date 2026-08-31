use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex; // 需要添加 tokio 依赖
use async_trait::async_trait;  // 导入宏

// ============ 1. 基础Trait定义 ============

/// 任务特质：所有任务必须实现的核心接口
// Rust 的 trait 目前不支持 async fn 语法，会编译报错。
// async fn 本质上是一个语法糖, 等价于返回 impl Future 的函数：fn f() -> impl Future<Output = ()> + Send { ... }
// impl Trait 是 Rust 中的不透明返回类型（opaque return type），
// 意思是：函数实际返回某个具体类型，但对外隐藏了它的真实身份，只承诺它满足指定的 trait。
// impl Future<Output = ()>：返回值实现了 Future trait，且该 Future 完成后产出 ()（即无返回值）。
// 本质上就是说 Rust 的 trait 目前不支持返回类型不明确的情况。
// 这个宏做了什么？ 它在底层把 trait 方法改写成了返回 Pin<Box<dyn Future>>（动态分发）
// #[async_trait] 宏（async-trait crate）的解决方案：
// 将用户写的 async fn 自动转换为返回 Pin<Box<dyn Future + ...>> 的同步方法。
// 1. Box 堆分配 Future；dyn Future 采用动态分发（带来微小运行时开销）；
// 2. Pin 约束满足 Future 需要固定内存地址的要求；
// 3. 默认生成的 trait object 附带 Send，可通过 #[async_trait(?Send)] 移除；
// 注意：该库**并未使用关联类型实现**；关联类型是静态分发的零开销备选方案，属于手动实现思路。
// 或者通过其他方式（如关联类型）来绕过“无法写出匿名类型”的限制。它本质上是将异步函数装箱（Box），
// 从而让类型变为可写的、已知的（Pin<Box<dyn Future>>）。
// Rust 的 Future 之所以需要关注内存地址，是因为 async/await 生成的 Future 内部可能包含“自引用”。
#[async_trait]  // 这个宏必须添加，因为 Task trait 中有 async fn,并且task被用于动态分发，如果全部静态分发就不用这个宏
pub trait Task: Send + Sync + Debug {

    // Send: 确保任务可以安全地转移到另一个线程执行
    // Sync: 确保任务可以安全地被多个线程同时访问
    // Debug: 确保任务可以打印调试信息(如果没有debug trait,无法使用 println! 和 dbg! 调试)
    // Eq: 确保可以比较是否相等(完全相等) (std::cmp::Eq 只有实现了 Eq 的类型，才能作为 HashMap 或 HashSet 的 Key)
    // std::hash::Hash: 确保可以哈希(将类型的实例转换为一个哈希值（通常是一个 u64 数字）) (std::hash::Hash)

    // 关联类型：任务的唯一标识
    type Id: Clone + Debug + Eq + std::hash::Hash + Send + Sync;
    // 关联类型：任务的执行结果
    type Output: Clone + Debug + Send + Sync;
    // 关联类型：任务的错误类型
    // type Error: Debug + Send + Sync + From<String>;
    // 修改点：简化 Error 约束，移除 From<String>，避免后续实现困难
//     原代码：type Error: ... + From<String>。这要求所有任务的错误类型都必须能从 String 转换而来。这对于标准库错误（如 std::io::Error）很难实现，导致代码难以编写。
// 修改后：type Error: ... + std::fmt::Display。这是更通用的做法，只要能打印错误信息即可
    type Error: Debug + Send + Sync + From<String> + std::fmt::Display;

    // 同步：线程被阻塞，CPU 空转等待
    // 异步：线程被释放，可以去执行其他任务，等结果回来再继续
    // async: 告诉编译器"这个函数包含异步操作，执行时可能会暂停，等待某个操作完成 再继续执行"

    // 必须实现的方法：执行任务
    async fn execute(&self) -> Result<Self::Output, Self::Error>;
    
    // 必须实现的方法：获取任务ID
    fn id(&self) -> Self::Id;
    
    // 必须实现的方法：获取任务优先级（数值越小优先级越高）
    fn priority(&self) -> u8;
    
    // 默认实现：获取任务超时时间
    fn timeout(&self) -> Duration {
        Duration::from_secs(30) // 默认30秒超时
    }
    
    // 默认实现：任务重试次数
    fn max_retries(&self) -> u8 {
        3
    }
    
    // 默认实现：任务名称
    fn name(&self) -> String {
        format!("Task-{:?}", self.id())
    }
    
    // 默认实现：检查任务是否可重试
    fn is_retryable(&self) -> bool {
        self.max_retries() > 0
    }
    
    // 默认实现：获取任务依赖（返回依赖的任务ID列表）
    fn dependencies(&self) -> Vec<Self::Id> {
        Vec::new() // 默认无依赖
    }
}

// ============ 2. Trait继承 ============

/// 可调度任务：继承Task，增加调度相关功能
pub trait ScheduledTask: Task {
    // 新增方法：调度时间
    // Instant: 用途：计算耗时、定时间隔，不受系统时间篡改影响，不会倒退
    fn scheduled_time(&self) -> Instant;
    
    // 新增方法：是否周期性任务
    fn is_periodic(&self) -> bool {
        false
    }
    
    // 新增方法：如果是周期性任务，获取执行间隔
    fn period(&self) -> Option<Duration> {
        if self.is_periodic() {
            Some(Duration::from_secs(60)) // 默认每分钟
        } else {
            None
        }
    }
}

/// 可监控任务：继承Task，增加监控功能
pub trait MonitorableTask: Task {
    // 获取任务进度（0-100）
    fn progress(&self) -> u8;
    
    // 获取任务状态描述
    fn status_description(&self) -> String;
    
    // 获取任务执行元数据
    fn metadata(&self) -> HashMap<String, String>;
}

// ============ 3. 具体实现：不同类型的任务 ============

// instant类型：用于测量时间间隔的单调递增时间点类型，只能计算时间差，不能表示具体日期时间。
// 就像一个只能往前走、不能回退的"秒表"计时器，用来测量代码跑了多久，而不是看现在是几点几分。
// 比如： Instant::now()：获取当前时间点
// ---------- 3.1 数据处理任务 ----------
#[derive(Debug, Clone)]
pub struct DataProcessingTask {
    id: String,
    data: Vec<u8>,
    priority: u8,
    retry_count: u8,
    start_time: Option<Instant>,
    progress: u8,
}

impl DataProcessingTask {
    pub fn new(id: String, data: Vec<u8>, priority: u8) -> Self {
        Self {
            id,
            data,
            priority,
            retry_count: 0,
            start_time: None,
            progress: 0,
        }
    }
}

#[async_trait]
impl Task for DataProcessingTask {
    type Id = String;
    type Output = Vec<u8>;
    type Error = String;

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        println!("[{}] 开始处理数据，大小: {} bytes", self.id, self.data.len());
        
        // 模拟复杂的数据处理
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // 模拟错误：如果数据为空
        if self.data.is_empty() {
            return Err("数据为空".to_string());
        }
        
        // 模拟数据处理：简单的转换
        // wrapping_add 是 u8 的回绕加法（溢出回绕，不会 panic）。
        let result: Vec<u8> = self.data.iter()
            .map(|b| b.wrapping_add(1))
            .collect();
        
        println!("[{}] 数据处理完成", self.id);
        Ok(result)
    }

    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    fn priority(&self) -> u8 {
        self.priority
    }
    
    fn timeout(&self) -> Duration {
        Duration::from_secs(10) // 数据处理10秒超时
    }
    
    fn max_retries(&self) -> u8 {
        5 // 数据任务重试5次
    }
}

// ---------- 3.2 HTTP请求任务 ----------
#[derive(Debug, Clone)]
pub struct HttpRequestTask {
    id: String,
    url: String,
    method: String,
    priority: u8,
    retry_count: u8,
}

impl HttpRequestTask {
    pub fn new(id: String, url: String, method: String, priority: u8) -> Self {
        Self {
            id,
            url,
            method,
            priority,
            retry_count: 0,
        }
    }
}

#[async_trait]
impl Task for HttpRequestTask {
    type Id = String;
    type Output = String;
    type Error = String;

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        println!("[{}] 发起HTTP请求: {} {}", self.id, self.method, self.url);
        
        // 模拟HTTP请求
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        if self.url.contains("error") {
            return Err(format!("请求失败: {}", self.url));
        }
        
        Ok(format!("Response from {}", self.url))
    }

    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    fn priority(&self) -> u8 {
        self.priority
    }
    
    fn timeout(&self) -> Duration {
        Duration::from_secs(5)
    }
    
    fn name(&self) -> String {
        format!("HTTP-{}-{}", self.method, self.id)
    }
}

impl ScheduledTask for HttpRequestTask {
    fn scheduled_time(&self) -> Instant {
        Instant::now() + Duration::from_secs(2)
    }
    
    fn is_periodic(&self) -> bool {
        self.url.contains("monitor") // 监控URL周期执行
    }
}

// ---------- 3.3 数据库备份任务（监控+调度） ----------
#[derive(Debug, Clone)]
pub struct DatabaseBackupTask {
    id: String,
    db_name: String,
    priority: u8,
    progress: u8,
    scheduled_at: Instant,
}

impl DatabaseBackupTask {
    pub fn new(id: String, db_name: String, priority: u8) -> Self {
        Self {
            id,
            db_name,
            priority,
            progress: 0,
            scheduled_at: Instant::now() + Duration::from_secs(5),
        }
    }
}

#[async_trait]
impl Task for DatabaseBackupTask {
    type Id = String;
    type Output = String;
    type Error = String;

    async fn execute(&self) -> Result<Self::Output, Self::Error> {
        println!("[{}] 开始备份数据库: {}", self.id, self.db_name);
        
        // 模拟备份过程
        for i in 0..=100 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            // 更新进度（实际中会使用可变状态）
            if i % 20 == 0 {
                println!("[{}] 备份进度: {}%", self.id, i);
            }
        }
        
        println!("[{}] 数据库备份完成", self.id);
        Ok(format!("Backup of {} completed", self.db_name))
    }

    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    fn priority(&self) -> u8 {
        self.priority
    }
    
    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

impl ScheduledTask for DatabaseBackupTask {
    fn scheduled_time(&self) -> Instant {
        self.scheduled_at
    }
    
    fn is_periodic(&self) -> bool {
        true
    }
    
    fn period(&self) -> Option<Duration> {
        Some(Duration::from_secs(3600)) // 每小时备份一次
    }
}

impl MonitorableTask for DatabaseBackupTask {
    fn progress(&self) -> u8 {
        self.progress
    }
    
    fn status_description(&self) -> String {
        format!("备份数据库 '{}'，进度: {}%", self.db_name, self.progress)
    }
    
    fn metadata(&self) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        meta.insert("database".to_string(), self.db_name.clone());
        meta.insert("type".to_string(), "backup".to_string());
        meta
    }
}

// ============ 4. 任务调度器（使用Trait作为泛型参数） ============
/// 任务调度器
// Mutex<Vec<T>>	互斥锁，保证同一时刻只有一个线程能访问这个 Vec
// Arc<Mutex<Vec<T>>>	原子引用计数，允许多个线程共享这个锁的所有权
// 为什么用 Mutex 而不是 RwLock？
// Mutex 提供独占访问，适合频繁修改的场景
// RwLock 允许多个读但只有一个写，但如果你频繁增删，Mutex 更简单高效
// RwLock 的实现比 Mutex 复杂得多，带来了额外开销，但读多写少的场合，并行效率提升
// 为什么用 Arc 而不是 Rc？
// Arc 是线程安全的，允许多个线程共享所有权
// rc 用于在单线程环境下实现多个所有者共享同一份数据 (rc会更快一些)
// Vec<(T::Id, T::Error)>	存储失败任务的 ID 和错误信息 组成的元组
pub struct TaskScheduler<T: Task> {
    tasks: Arc<Mutex<Vec<T>>>,
    completed: Arc<Mutex<Vec<T::Output>>>,
    failed: Arc<Mutex<Vec<(T::Id, T::Error)>>>,
}

impl<T: Task> TaskScheduler<T> {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            completed: Arc::new(Mutex::new(Vec::new())),
            failed: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    // 添加任务
    pub async fn add_task(&self, task: T) {
        let mut tasks = self.tasks.lock().await;
        tasks.push(task);
        // 按优先级排序
        tasks.sort_by_key(|t| t.priority());
    }
    
    // 执行所有任务
    pub async fn execute_all(&self) {
    // 1. 获取锁
    let tasks = {
        let mut tasks = self.tasks.lock().await;

        // lock之后，返回的tasks是 MutexGuard，它不是 Send，不能在 .await 期间持有，所以需要取走所有任务，之后遍历
        // 在 Rust 异步编程中，.await 点可能发生线程切换
        // MutexGuard 包含借用引用（&'a Mutex<T>），它被设计为绑定到创建它的线程：

        // 取走所有任务，留空 Vec
        // 之所以这么做，是为了防止在锁内做耗时操作，耗时操作会导致锁长时间被占用，影响其他任务
        std::mem::take(&mut *tasks)  
    };  // 锁在这里释放（tasks 守卫离开作用域）
    
    // 2. 在锁外执行耗时操作
    for task in tasks {  // tasks 是 Vec<T>，不持有锁
        self.execute_with_retry(task).await;  // 可以安全 await
    }
}
    
    // 带重试的任务执行
    async fn execute_with_retry(&self, task: T) {
        let max_retries = task.max_retries();
        
        for attempt in 0..=max_retries {
            let task_id = task.id().clone();

            if attempt > 0 {
                println!("[任务 {:?}] 第 {} 次重试", task_id, attempt);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            
            // 使用超时执行
            let result = tokio::time::timeout(task.timeout(), task.execute()).await;
            
            match result {
                Ok(Ok(output)) => {
                    println!("[任务 {:?}] 执行成功", task_id);
                    let mut completed = self.completed.lock().await;
                    completed.push(output);
                    return;
                }
                Ok(Err(e)) => {
                    println!("[任务 {:?}] 执行失败: {:?}", task_id, e);
                    if attempt == max_retries as u8 {
                        let mut failed = self.failed.lock().await;
                        failed.push((task_id, e));
                    }
                }
                Err(_) => {
                    println!("[任务 {:?}] 超时", task_id);
                    if attempt == max_retries as u8 {
                        let mut failed = self.failed.lock().await;

                        // 将字符串 "超时" 转换为 Error 类型。
                        // failed 的类型是 Vec<(Id, Error)>，push 要求第二个元素是 Error 类型。
                        // 由于 Error 实现了 From<String>，编译器会自动推导出 into() 将 String 转换为 Error。
                        // 等价于：Error::from("超时".to_string())
                        failed.push((task_id, "超时".to_string().into()));

                        // ✅ 另一种正确的写法
                        // 因为 Error 是关联类型，需要加 T:: 前缀
                        // failed.push((task_id, T::Error::from("超时".to_string())));
                    }
                }
            }
        }
    }
    
    // 获取统计信息
    pub async fn get_stats(&self) -> TaskStats {
        let completed_count = self.completed.lock().await.len();
        let failed_count = self.failed.lock().await.len();
        
        TaskStats {
            completed: completed_count,
            failed: failed_count,
            pending: self.tasks.lock().await.len(),
        }
    }
}

// 统计信息
#[derive(Debug, Clone)]
pub struct TaskStats {
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
}

// ============ 5. 使用Trait对象（动态分发） ============

/// 任务管理器：使用Trait对象存储不同类型的任务
pub struct TaskManager {
    // 动态分发，可以存放任何实现了 Task trait 的类型，但这些类型的 Id、Output、Error 都必须是 String。
    tasks: Arc<Mutex<Vec<Box<dyn Task<Id = String, Output = String, Error = String>>>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    // 添加不同类型的任务到同一个容器
    pub async fn add_task(&self, task: Box<dyn Task<Id = String, Output = String, Error = String>>) {
        let mut tasks = self.tasks.lock().await;
        tasks.push(task);
    }
    
    // 执行所有任务
    pub async fn execute_all(&self) {
        let tasks = {
            let mut tasks = self.tasks.lock().await;
            // // 取走所有任务，留空 Vec
            std::mem::take(&mut *tasks)
        };
        
        for task in tasks {
            // 使用动态分发
            let _ = task.execute().await;
        }
    }
}

// ============ 6. 主函数演示 ============

#[tokio::main]
async fn main() {
    println!("===== Rust Trait 完整示例：分布式任务调度系统 =====\n");
    
    // 6.1 演示泛型调度器
    println!("1. 泛型调度器演示");
    let scheduler = TaskScheduler::<DataProcessingTask>::new();
    
    let task1 = DataProcessingTask::new(
        "task-001".to_string(),
        vec![1, 2, 3, 4, 5],
        2,
    );
    
    let task2 = DataProcessingTask::new(
        "task-002".to_string(),
        vec![10, 20, 30],
        1, // 更高优先级
    );
    
    // 添加一个会失败的任务
    let task3 = DataProcessingTask::new(
        "task-003".to_string(),
        vec![], // 空数据会导致失败
        3,
    );
    
    scheduler.add_task(task1).await;
    scheduler.add_task(task2).await;
    scheduler.add_task(task3).await;
    
    scheduler.execute_all().await;
    
    let stats = scheduler.get_stats().await;
    println!("统计信息: {:?}\n", stats);
    
    // 6.2 演示Trait对象
    println!("2. Trait对象演示");
    let manager = TaskManager::new();
    
    // 创建不同类型的任务但使用统一的Trait对象
    let http_task = Box::new(HttpRequestTask::new(
        "1001".to_string(),
        "https://api.example.com/data".to_string(),
        "GET".to_string(),
        1,
    ));
    
    let backup_task = Box::new(DatabaseBackupTask::new(
        "backup-001".to_string(),
        "production_db".to_string(),
        2,
    ));
    
    manager.add_task(http_task).await;
    manager.add_task(backup_task).await;
    
    manager.execute_all().await;
    
    // 6.3 演示Trait继承
    println!("\n3. Trait继承演示");
    let scheduled_task = HttpRequestTask::new(
        "1002".to_string(),
        "https://monitor.example.com/health".to_string(),
        "GET".to_string(),
        1,
    );
    
    // 使用 ScheduledTask trait 的方法
    if scheduled_task.is_periodic() {
        println!("任务是周期性执行，间隔: {:?}", scheduled_task.period());
    }
    println!("调度时间: {:?}", scheduled_task.scheduled_time());
    
    // 6.4 演示完整功能
    println!("\n4. 完整功能演示 - 数据库备份任务");
    let backup_task = DatabaseBackupTask::new(
        "backup-002".to_string(),
        "analytics_db".to_string(),
        1,
    );
    
    // Task trait 方法
    println!("任务ID: {:?}", backup_task.id());
    println!("任务名称: {}", backup_task.name());
    println!("任务优先级: {}", backup_task.priority());
    println!("最大重试次数: {}", backup_task.max_retries());
    println!("超时时间: {:?}", backup_task.timeout());
    
    // ScheduledTask trait 方法
    println!("是周期性任务: {}", backup_task.is_periodic());
    if backup_task.is_periodic() {
        println!("执行周期: {:?}", backup_task.period());
    }
    
    // MonitorableTask trait 方法
    println!("任务进度: {}%", backup_task.progress());
    println!("状态描述: {}", backup_task.status_description());
    println!("元数据: {:?}", backup_task.metadata());
    
    // 执行任务
    let result = backup_task.execute().await;
    println!("执行结果: {:?}", result);
}

// ============ 7. 关联类型和泛型约束的高级用法 ============
// todo
/// 任务处理器：使用高阶Trait约束
// [async_trait]这个宏会将你写的 async fn 方法签名，转换为一个返回 Pin<Box<dyn Future<Output = ...> + Send + '_>> 的方法，
// 从而允许通过 dyn Trait 来使用
// 什么时候才需要 #[async_trait]？场景 1：需要动态分发（trait object）场景 2：需要向后兼容（Rust 1.75 之前）
// TaskProcessor<T: Task>：这是一个泛型 trait，T 必须实现 Task trait
// 在这个trait中，会有也必须有函数使用到这个泛型

// #[async_trait] // ← 在旧版本中需要这个宏（Rust 1.75 以下）
pub trait TaskProcessor<T: Task>: Send + Sync {

    // Send：表示所有权可以在线程间转移
    // Sync：表示不可变引用 &T 可以在线程间共享

    // 使用关联类型
    type Context: Clone + Send + Sync;
    type Result: Send;
    
    // 处理任务 - 使用显式生命周期
    // 返回:表示函数返回一个实现了 Future trait 的类型，并且这个 Future 可以安全地在线程间传递。
    // Future 是 Rust 异步编程的核心 trait，表示一个可能尚未完成的计算
    // 这两个生命周期 'a 和 'b 的核心作用是：告诉编译器，返回的 Future 借用了 self、task 和 context 的引用，
    // 它的存活时间不能超过这些引用的最短生命周期。
    // 为什么不都用 'a或者 'b呢:强制它们一样长，会严重限制你的代码灵活性
    // 如果task也用 'a ，那么 task 的生命周期就必须和 self 的生命周期一样长，这显然是不合理的。
    fn process<'a, 'b>(
        &'a self, 
        task: &'b T, 
        context: &'b Self::Context
    ) -> impl std::future::Future<Output = Self::Result> + Send;

    // 如果这样写，就需要 #[async_trait]（Rust 1.75 以下）
    // async fn process<'a, 'b>(
    //     &'a self, 
    //     task: &'b T, 
    //     context: &'b Self::Context
    // ) -> Self::Result ;

    // 注意：不能混用以上两种写法：（async fn 和 impl Trait）
    // 当你在一个 trait 中同时使用这两种写法时：
    // async fn process(...) -> Self::Result 实际上是 
    // fn process(...) -> impl Future<Output = Self::Result> 的语法糖。
    // 这个糖没有指明返回的 Future 是否实现了 Send 或拥有特定的生命周期。
    // 而 fn process_batch(...) -> impl Future<Output = Vec<Self::Result>> + Send 
    // 则明确指定了返回的 Future 必须实现 Send。
    // 这种不统一会导致编译器在推导 trait 的“核心类型”时产生困惑。你的 trait 既包含了“未约束”的 Future（async fn），
    // 又包含了“已约束”的 Future（impl Future + Send），这使得 trait 作为一个整体变得不可预测，
    // 进而破坏了其作为 trait object 的一致性。
    
    // 批量处理
    fn process_batch<'a, 'b>(
        &'a self, 
        tasks: &'b [T], 
        context: &'b Self::Context
    ) -> impl std::future::Future<Output = Vec<Self::Result>> + Send
    where
        T: Clone,
        Self: Send,
    {
        async move {
            let mut results = Vec::new();
            for task in tasks.iter() {
                let res = self.process(task, context).await;
                results.push(res);
            }
            results
        }
    }
}

/// 示例实现：日志处理器
pub struct LoggingTaskProcessor;

impl<T: Task<Output = String>> TaskProcessor<T> for LoggingTaskProcessor {
    type Context = String; // 日志前缀
    type Result = String; // 日志输出
    
    fn process<'a, 'b>(
        &'a self, 
        task: &'b T, 
        context: &'b Self::Context
    ) -> impl std::future::Future<Output = Self::Result> + Send {
        async move {
            let execution_result = task.execute().await;

            match execution_result {
                Ok(output) => {
                    format!("[{}] ✅ 任务 {:?} 成功，输出: {}", context, task.id(), output)
                }
                Err(e) => {
                    format!("[{}] ❌ 任务 {:?} 失败，错误: {}", context, task.id(), e)
                }
            }
        }
    }
}
// ============ 8. 条件编译和派生Trait ============

// 使用条件编译：仅在测试时启用
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_task_execution() {
        let task = DataProcessingTask::new(
            "test-001".to_string(),
            vec![1, 2, 3],
            1,
        );
        
        let result = task.execute().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![2, 3, 4]);
    }
    
    #[tokio::test]
    async fn test_scheduler() {
        let scheduler = TaskScheduler::<DataProcessingTask>::new();
        let task = DataProcessingTask::new(
            "test-002".to_string(),
            vec![5, 6, 7],
            1,
        );
        
        scheduler.add_task(task).await;
        scheduler.execute_all().await;
        
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.completed, 1);
    }
}