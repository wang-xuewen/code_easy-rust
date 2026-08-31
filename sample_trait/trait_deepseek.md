我来设计一个**分布式任务调度系统**的核心组件，用Trait来展示其完整特性。这个例子包含了Trait定义、默认实现、继承、关联类型、动态分发、对象安全等所有重要概念。

## 完整示例：分布式任务调度系统

```rust
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex; // 需要添加 tokio 依赖

// ============ 1. 基础Trait定义 ============

/// 任务特质：所有任务必须实现的核心接口
pub trait Task: Send + Sync + Debug {
    // 关联类型：任务的唯一标识
    type Id: Clone + Debug + Eq + std::hash::Hash + Send + Sync;
    // 关联类型：任务的执行结果
    type Output: Clone + Debug + Send + Sync;
    // 关联类型：任务的错误类型
    type Error: Debug + Send + Sync;

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
    id: u64,
    url: String,
    method: String,
    priority: u8,
    retry_count: u8,
}

impl HttpRequestTask {
    pub fn new(id: u64, url: String, method: String, priority: u8) -> Self {
        Self {
            id,
            url,
            method,
            priority,
            retry_count: 0,
        }
    }
}

impl Task for HttpRequestTask {
    type Id = u64;
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
        self.id
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
        let tasks = {
            let mut tasks = self.tasks.lock().await;
            std::mem::take(&mut *tasks)
        };
        
        for task in tasks {
            self.execute_with_retry(task).await;
        }
    }
    
    // 带重试的任务执行
    async fn execute_with_retry(&self, mut task: T) {
        let max_retries = task.max_retries();
        let task_id = task.id();
        
        for attempt in 0..=max_retries {
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
                        failed.push((task_id, "超时".to_string() as T::Error));
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
        1001,
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
        1002,
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

/// 任务处理器：使用高阶Trait约束
pub trait TaskProcessor<T: Task> {
    // 使用关联类型
    type Context: Clone + Send + Sync;
    type Result;
    
    // 处理任务
    fn process(&self, task: &T, context: &Self::Context) -> Self::Result;
    
    // 批量处理
    fn process_batch(&self, tasks: &[T], context: &Self::Context) -> Vec<Self::Result>
    where
        T: Clone, // 额外的Trait约束
    {
        tasks.iter().map(|t| self.process(t, context)).collect()
    }
}

/// 示例实现：日志处理器
pub struct LoggingTaskProcessor;

impl<T: Task<Output = String>> TaskProcessor<T> for LoggingTaskProcessor {
    type Context = String; // 日志前缀
    type Result = String; // 日志输出
    
    fn process(&self, task: &T, context: &Self::Context) -> Self::Result {
        format!(
            "[{}] 处理任务: {:?}, 输出: {:?}",
            context, task.id(), task.execute()
        )
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
```

## 关键概念总结

### 1. **Trait定义**
- 定义接口契约，包含必须实现和默认实现的方法
- 使用关联类型（`type Id`）实现类型灵活绑定

### 2. **Trait继承**
- `ScheduledTask: Task` 和 `MonitorableTask: Task`
- 子Trait继承父Trait的所有方法，可添加新方法

### 3. **泛型约束**
- `impl<T: Task>` 约束泛型参数必须实现Task Trait
- 在结构体中使用泛型参数

### 4. **Trait对象（动态分发）**
- `Box<dyn Task<Id = String, Output = String, Error = String>>`
- 允许在容器中存储不同类型的任务

### 5. **关联类型**
- 每个Task实现可以定义自己的Id、Output、Error类型
- 提供类型安全的接口

### 6. **默认实现**
- `timeout()`, `max_retries()`, `name()` 等方法提供默认行为
- 子类型可以覆盖

### 7. **async/await支持**
- Trait方法可以是异步的（需要`async-trait`或Rust 1.75+）

### 8. **生产级特性**
- 超时控制
- 重试机制
- 优先级调度
- 进度监控
- 错误处理
- 统计信息

这个示例覆盖了Rust Trait的几乎所有重要特性，并且设计模式可以直接用于实际的生产系统。你可以根据具体需求调整和扩展这些Trait来构建自己的系统。