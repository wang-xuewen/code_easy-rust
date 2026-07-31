use std::collections::HashMap;
use std::fmt::Display;

// 1. 定义错误类型，并让它实现 Display（为后续 Trait 继承做准备）
#[derive(Debug)]
enum CacheError {
    NotFound,
    SerializationFailed,
}

impl Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cache Error: {:?}", self)
    }
}

// 2. 定义核心 Trait：使用关联类型（Associated Types）
// 关联类型让 Cache 的 Key 和 Value 成为实现的一部分，比泛型参数更简洁
trait Cache {
    type Key;
    type Value;
    type Error;

    // 必须实现的方法
    fn get(&self, key: &Self::Key) -> Result<Self::Value, Self::Error>;
    fn set(&mut self, key: Self::Key, value: Self::Value) -> Result<(), Self::Error>;

    // 默认实现：提供通用的批量获取逻辑，减少重复代码
    fn batch_get(&self, keys: &[Self::Key]) -> Vec<Result<Self::Value, Self::Error>> 
    where 
        Self::Key: Clone 
    {
        keys.iter().map(|k| self.get(k)).collect()
    }
}

// 3. Trait 继承（Supertraits）
// 要求任何实现了 Cache 的类型，也必须实现 Display，方便打印缓存状态
trait InspectableCache: Cache + Display {
    fn cache_name(&self) -> &str;
}

// 4. 具体实现：内存缓存
struct MemoryCache {
    store: HashMap<String, String>,
}

impl MemoryCache {
    fn new() -> Self {
        Self { store: HashMap::new() }
    }
}

impl Cache for MemoryCache {
    type Key = String;
    type Value = String;
    type Error = CacheError;

    fn get(&self, key: &String) -> Result<String, CacheError> {
        self.store.get(key).cloned().ok_or(CacheError::NotFound)
    }

    fn set(&mut self, key: String, value: String) -> Result<(), CacheError> {
        self.store.insert(key, value);
        Ok(())
    }
}

// 实现 Display 以满足 InspectableCache 的继承约束
impl Display for MemoryCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MemoryCache (size: {})", self.store.len())
    }
}

// 实现 InspectableCache
impl InspectableCache for MemoryCache {
    fn cache_name(&self) -> &str {
        "In-Memory Cache"
    }
}

// 5. 业务层：使用 Trait 对象（动态分发）
// 在运行时，业务层不需要知道底层是内存还是 Redis，实现了真正的解耦
struct UserService {
    // 使用 Box<dyn Trait> 在堆上分配，支持运行时多态
    cache: Box<dyn InspectableCache<Key = String, Value = String, Error = CacheError>>,
}

impl UserService {
    fn new(cache: Box<dyn InspectableCache<Key = String, Value = String, Error = CacheError>>) -> Self {
        println!("Initializing service with: {}", cache); // 调用 Display trait
        Self { cache }
    }

    fn get_user(&self, user_id: &str) -> String {
        match self.cache.get(&user_id.to_string()) {
            Ok(val) => val,
            Err(_) => format!("User {} not in cache", user_id),
        }
    }
}

fn main() {
    // 初始化具体的缓存实现
    let mut mem_cache = MemoryCache::new();
    mem_cache.set("user:1001".to_string(), "Alice".to_string()).unwrap();

    // 注入到业务服务中（动态分发）
    let service = UserService::new(Box::new(mem_cache));
    
    println!("Result: {}", service.get_user("user:1001")); // 命中缓存
    println!("Result: {}", service.get_user("user:9999")); // 未命中
}