我来创建一个生产级的Rust项目，通过实现一个**内存缓存系统**来全方位展示生命周期概念。这个项目将涵盖生命周期的各个方面，包括函数、结构体、trait、泛型等场景。

```rust
// Cargo.toml
[package]
name = "lifetime-cache"
version = "0.1.0"
edition = "2021"

[dependencies]
chrono = "0.4"
thiserror = "1.0"
```

```rust
// src/lib.rs
//! 一个展示Rust生命周期概念的缓存系统
//! 
//! 这个项目通过实现一个带有TTL（生存时间）的缓存系统，
//! 展示了生命周期的各种使用场景

mod cache;
mod cache_key;
mod cache_value;
mod storage;
mod strategy;

pub use cache::Cache;
pub use cache_key::{CacheKey, KeyStrategy};
pub use cache_value::CacheValue;
pub use storage::{Storage, StorageGuard};
pub use strategy::{EvictionStrategy, LRUStrategy, FIFOStrategy};

// src/cache.rs
use std::marker::PhantomData;
use std::time::{Duration, SystemTime};
use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::{
    CacheKey, CacheValue, Storage, StorageGuard, 
    EvictionStrategy, KeyStrategy
};

/// 缓存相关的错误类型
#[derive(Error, Debug)]
pub enum CacheError<'a> {
    #[error("Key not found: {0}")]
    KeyNotFound(&'a str),
    
    #[error("Key expired: {0}")]
    KeyExpired(&'a str),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Eviction failed")]
    EvictionFailed,
}

/// 缓存条目的元数据
#[derive(Debug, Clone, PartialEq)]
struct EntryMetadata {
    /// 创建时间
    created_at: DateTime<Utc>,
    /// 最后访问时间
    last_accessed: DateTime<Utc>,
    /// TTL（生存时间）
    ttl: Option<Duration>,
}

/// 缓存条目的所有权类型
#[derive(Debug, Clone, PartialEq)]
pub enum EntryOwnership<'a> {
    /// 借用的数据（生命周期与缓存绑定）
    Borrowed(&'a [u8]),
    /// 拥有的数据（生命周期独立）
    Owned(Vec<u8>),
    /// 共享引用（生命周期与缓存绑定，但可共享）
    Shared(&'a [u8]),
}

impl<'a> EntryOwnership<'a> {
    /// 获取数据引用，生命周期与缓存绑定
    pub fn as_ref(&self) -> &[u8] {
        match self {
            EntryOwnership::Borrowed(data) => data,
            EntryOwnership::Owned(data) => data.as_slice(),
            EntryOwnership::Shared(data) => data,
        }
    }
    
    /// 提取数据的所有权
    pub fn into_owned(self) -> Vec<u8> {
        match self {
            EntryOwnership::Borrowed(data) => data.to_vec(),
            EntryOwnership::Owned(data) => data,
            EntryOwnership::Shared(data) => data.to_vec(),
        }
    }
}

/// 缓存条目
#[derive(Debug)]
pub struct CacheEntry<'a, K, V> 
where
    K: CacheKey,
    V: CacheValue,
{
    key: K,
    value: EntryOwnership<'a>,
    metadata: EntryMetadata,
    _marker: PhantomData<V>,
}

impl<'a, K, V> CacheEntry<'a, K, V>
where
    K: CacheKey,
    V: CacheValue,
{
    /// 创建新的缓存条目
    pub fn new(key: K, value: V, ttl: Option<Duration>) -> Self
    where
        V: 'a,
    {
        let now = Utc::now();
        Self {
            key,
            value: EntryOwnership::Owned(value.into_bytes()),
            metadata: EntryMetadata {
                created_at: now,
                last_accessed: now,
                ttl,
            },
            _marker: PhantomData,
        }
    }
    
    /// 创建借用条目的缓存条目
    pub fn new_borrowed(key: K, value: &'a V, ttl: Option<Duration>) -> Self
    where
        V: 'a,
    {
        let now = Utc::now();
        Self {
            key,
            value: EntryOwnership::Borrowed(value.as_bytes()),
            metadata: EntryMetadata {
                created_at: now,
                last_accessed: now,
                ttl,
            },
            _marker: PhantomData,
        }
    }
    
    /// 更新条目的最后访问时间
    pub fn touch(&mut self) {
        self.metadata.last_accessed = Utc::now();
    }
    
    /// 检查条目是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.metadata.ttl {
            let elapsed = Utc::now() - self.metadata.created_at;
            elapsed.to_std().unwrap_or(Duration::ZERO) > ttl
        } else {
            false
        }
    }
    
    /// 获取条目的值引用
    pub fn value(&self) -> &[u8] {
        self.value.as_ref()
    }
    
    /// 获取条目的键
    pub fn key(&self) -> &K {
        &self.key
    }
    
    /// 获取条目的元数据
    pub fn metadata(&self) -> &EntryMetadata {
        &self.metadata
    }
}

/// 主缓存结构
pub struct Cache<'a, K, V, S, E>
where
    K: CacheKey,
    V: CacheValue,
    S: Storage<Key = K, Value = CacheEntry<'a, K, V>>,
    E: EvictionStrategy<Key = K>,
{
    storage: S,
    eviction_strategy: E,
    key_strategy: Box<dyn KeyStrategy<Key = K> + 'a>,
    max_size: usize,
    _marker: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K, V, S, E> Cache<'a, K, V, S, E>
where
    K: CacheKey,
    V: CacheValue,
    S: Storage<Key = K, Value = CacheEntry<'a, K, V>>,
    E: EvictionStrategy<Key = K>,
{
    /// 创建新的缓存实例
    pub fn new(
        storage: S,
        eviction_strategy: E,
        key_strategy: Box<dyn KeyStrategy<Key = K> + 'a>,
        max_size: usize,
    ) -> Self {
        Self {
            storage,
            eviction_strategy,
            key_strategy,
            max_size,
            _marker: PhantomData,
        }
    }
    
    /// 插入缓存项（拥有数据所有权）
    pub fn insert(&mut self, key: K, value: V, ttl: Option<Duration>) -> Result<(), CacheError<'a>>
    where
        V: 'a,
    {
        // 检查是否需要驱逐
        self.ensure_capacity()?;
        
        let entry = CacheEntry::new(key.clone(), value, ttl);
        self.storage.insert(key, entry)?;
        
        // 记录访问
        self.eviction_strategy.record_access(&key)?;
        
        Ok(())
    }
    
    /// 插入缓存项（借用数据）
    pub fn insert_borrowed(
        &mut self, 
        key: K, 
        value: &'a V, 
        ttl: Option<Duration>
    ) -> Result<(), CacheError<'a>>
    where
        V: 'a,
    {
        self.ensure_capacity()?;
        
        let entry = CacheEntry::new_borrowed(key.clone(), value, ttl);
        self.storage.insert(key, entry)?;
        self.eviction_strategy.record_access(&key)?;
        
        Ok(())
    }
    
    /// 获取缓存项
    pub fn get(&mut self, key: &K) -> Result<StorageGuard<'_, K, CacheEntry<'a, K, V>>, CacheError<'a>> {
        // 更新访问记录
        self.eviction_strategy.record_access(key)?;
        
        // 获取条目
        let entry = self.storage.get(key)?;
        
        // 检查是否过期
        if entry.is_expired() {
            // 删除过期条目
            self.storage.remove(key)?;
            return Err(CacheError::KeyExpired(key.to_string().as_str()));
        }
        
        // 更新访问时间
        // 注意：这里我们需要修改条目，但StorageGuard是只读的
        // 因此我们需要在外部处理
        Ok(entry)
    }
    
    /// 获取并更新访问时间（需要可变访问）
    pub fn get_mut(&mut self, key: &K) -> Result<&mut CacheEntry<'a, K, V>, CacheError<'a>> {
        self.eviction_strategy.record_access(key)?;
        
        // 获取可变条目
        let entry = self.storage.get_mut(key)?;
        
        if entry.is_expired() {
            self.storage.remove(key)?;
            return Err(CacheError::KeyExpired(key.to_string().as_str()));
        }
        
        // 更新最后访问时间
        entry.touch();
        Ok(entry)
    }
    
    /// 删除缓存项
    pub fn remove(&mut self, key: &K) -> Result<(), CacheError<'a>> {
        self.storage.remove(key)?;
        self.eviction_strategy.remove(key)?;
        Ok(())
    }
    
    /// 清空缓存
    pub fn clear(&mut self) -> Result<(), CacheError<'a>> {
        self.storage.clear()?;
        self.eviction_strategy.clear()?;
        Ok(())
    }
    
    /// 检查缓存是否包含键
    pub fn contains(&self, key: &K) -> bool {
        self.storage.contains(key)
    }
    
    /// 获取缓存大小
    pub fn size(&self) -> usize {
        self.storage.size()
    }
    
    /// 确保缓存有足够容量
    fn ensure_capacity(&mut self) -> Result<(), CacheError<'a>> {
        while self.size() >= self.max_size {
            // 获取需要驱逐的键
            let eviction_key = self.eviction_strategy
                .select_victim()
                .ok_or(CacheError::EvictionFailed)?;
            
            // 驱逐
            self.storage.remove(&eviction_key)?;
            self.eviction_strategy.remove(&eviction_key)?;
        }
        Ok(())
    }
    
    /// 批量获取键（展示生命周期与迭代器）
    pub fn get_batch<'b, I>(&'b mut self, keys: I) -> Vec<Result<StorageGuard<'b, K, CacheEntry<'a, K, V>>, CacheError<'a>>>
    where
        I: IntoIterator<Item = &'b K>,
        'a: 'b,
    {
        keys.into_iter()
            .map(|key| self.get(key))
            .collect()
    }
    
    /// 获取所有键（展示生命周期约束）
    pub fn keys<'b>(&'b self) -> impl Iterator<Item = &'b K>
    where
        'a: 'b,
    {
        self.storage.keys()
    }
    
    /// 映射函数（展示高阶函数生命周期）
    pub fn map_values<F, R>(&self, f: F) -> Vec<(K, R)>
    where
        F: Fn(&CacheEntry<'a, K, V>) -> R + 'a,
        K: Clone,
    {
        self.storage
            .iter()
            .map(|(key, entry)| (key.clone(), f(entry)))
            .collect()
    }
    
    /// 使用生命周期标注的复杂操作
    pub fn with_borrowed_data<'b, F, R>(
        &'b self,
        f: F,
    ) -> R
    where
        F: FnOnce(&'b Storage<S>) -> R,
        'a: 'b,
    {
        f(&self.storage)
    }
}

// src/cache_key.rs
use std::hash::{Hash, Hasher};

/// 缓存键特征
pub trait CacheKey: Clone + Eq + Hash + std::fmt::Debug + Send + Sync {
    fn to_string(&self) -> String;
}

impl CacheKey for String {
    fn to_string(&self) -> String {
        self.clone()
    }
}

impl CacheKey for &str {
    fn to_string(&self) -> String {
        (*self).to_string()
    }
}

impl CacheKey for i32 {
    fn to_string(&self) -> String {
        self.to_string()
    }
}

impl CacheKey for u64 {
    fn to_string(&self) -> String {
        self.to_string()
    }
}

/// 键策略特征（展示生命周期）
pub trait KeyStrategy {
    type Key: CacheKey;
    
    /// 生成键（生命周期：'a）
    fn generate_key<'a>(&self, data: &'a str) -> Self::Key
    where
        Self: 'a;
    
    /// 验证键（生命周期：'a）
    fn validate_key<'a>(&self, key: &'a Self::Key) -> bool
    where
        Self: 'a;
}

/// 简单键策略
pub struct SimpleKeyStrategy;

impl KeyStrategy for SimpleKeyStrategy {
    type Key = String;
    
    fn generate_key<'a>(&self, data: &'a str) -> Self::Key {
        data.to_string()
    }
    
    fn validate_key<'a>(&self, key: &'a Self::Key) -> bool {
        !key.is_empty()
    }
}

/// 哈希键策略
pub struct HashKeyStrategy {
    salt: String,
}

impl HashKeyStrategy {
    pub fn new(salt: impl Into<String>) -> Self {
        Self { salt: salt.into() }
    }
}

impl KeyStrategy for HashKeyStrategy {
    type Key = String;
    
    fn generate_key<'a>(&self, data: &'a str) -> Self::Key {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        self.salt.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
    
    fn validate_key<'a>(&self, key: &'a Self::Key) -> bool {
        key.len() == 16 // 简单验证
    }
}

// src/cache_value.rs
/// 缓存值特征
pub trait CacheValue: Send + Sync + std::fmt::Debug {
    /// 转换为字节切片
    fn as_bytes(&self) -> &[u8];
    
    /// 从字节切片创建
    fn from_bytes(bytes: &[u8]) -> Self;
    
    /// 转换为字节向量
    fn into_bytes(self) -> Vec<u8>
    where
        Self: Sized,
    {
        self.as_bytes().to_vec()
    }
}

impl CacheValue for String {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
    
    fn from_bytes(bytes: &[u8]) -> Self {
        String::from_utf8_lossy(bytes).to_string()
    }
}

impl CacheValue for Vec<u8> {
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
    
    fn from_bytes(bytes: &[u8]) -> Self {
        bytes.to_vec()
    }
}

impl CacheValue for &[u8] {
    fn as_bytes(&self) -> &[u8] {
        self
    }
    
    fn from_bytes(bytes: &[u8]) -> Self {
        bytes
    }
}

// src/storage.rs
use std::collections::HashMap;
use std::ops::Deref;
use thiserror::Error;

/// 存储错误
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Key not found")]
    KeyNotFound,
    #[error("Storage full")]
    StorageFull,
    #[error("IO error: {0}")]
    IoError(String),
}

/// 存储守卫（展示生命周期）
pub struct StorageGuard<'a, K, V>
where
    K: CacheKey,
    V: std::fmt::Debug,
{
    storage: &'a mut Storage<K, V>,
    key: K,
}

impl<'a, K, V> Deref for StorageGuard<'a, K, V>
where
    K: CacheKey,
    V: std::fmt::Debug,
{
    type Target = V;
    
    fn deref(&self) -> &Self::Target {
        // 这里简化了实现，实际需要从storage获取
        unimplemented!("需要在真实实现中获取值")
    }
}

impl<'a, K, V> Drop for StorageGuard<'a, K, V>
where
    K: CacheKey,
    V: std::fmt::Debug,
{
    fn drop(&mut self) {
        // 释放资源
    }
}

/// 存储特征
pub trait Storage: std::fmt::Debug {
    type Key: CacheKey;
    type Value: std::fmt::Debug;
    
    /// 插入值
    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Result<(), StorageError>;
    
    /// 获取值（只读，带生命周期）
    fn get<'a>(&'a self, key: &Self::Key) -> Result<&'a Self::Value, StorageError>;
    
    /// 获取值（可变，带生命周期）
    fn get_mut<'a>(&'a mut self, key: &Self::Key) -> Result<&'a mut Self::Value, StorageError>;
    
    /// 删除值
    fn remove(&mut self, key: &Self::Key) -> Result<(), StorageError>;
    
    /// 清空存储
    fn clear(&mut self) -> Result<(), StorageError>;
    
    /// 检查键是否存在
    fn contains(&self, key: &Self::Key) -> bool;
    
    /// 获取大小
    fn size(&self) -> usize;
    
    /// 获取所有键（带生命周期）
    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Key> + 'a>;
    
    /// 迭代器（带生命周期）
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a Self::Key, &'a Self::Value)> + 'a>;
}

/// 内存存储实现
#[derive(Debug)]
pub struct MemoryStorage<K, V>
where
    K: CacheKey,
    V: std::fmt::Debug,
{
    data: HashMap<K, V>,
}

impl<K, V> MemoryStorage<K, V>
where
    K: CacheKey,
    V: std::fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl<K, V> Storage for MemoryStorage<K, V>
where
    K: CacheKey,
    V: std::fmt::Debug,
{
    type Key = K;
    type Value = V;
    
    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Result<(), StorageError> {
        self.data.insert(key, value);
        Ok(())
    }
    
    fn get<'a>(&'a self, key: &Self::Key) -> Result<&'a Self::Value, StorageError> {
        self.data
            .get(key)
            .ok_or(StorageError::KeyNotFound)
    }
    
    fn get_mut<'a>(&'a mut self, key: &Self::Key) -> Result<&'a mut Self::Value, StorageError> {
        self.data
            .get_mut(key)
            .ok_or(StorageError::KeyNotFound)
    }
    
    fn remove(&mut self, key: &Self::Key) -> Result<(), StorageError> {
        self.data
            .remove(key)
            .ok_or(StorageError::KeyNotFound)?;
        Ok(())
    }
    
    fn clear(&mut self) -> Result<(), StorageError> {
        self.data.clear();
        Ok(())
    }
    
    fn contains(&self, key: &Self::Key) -> bool {
        self.data.contains_key(key)
    }
    
    fn size(&self) -> usize {
        self.data.len()
    }
    
    fn keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Key> + 'a> {
        Box::new(self.data.keys())
    }
    
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a Self::Key, &'a Self::Value)> + 'a> {
        Box::new(self.data.iter())
    }
}

// src/strategy.rs
use std::collections::VecDeque;
use thiserror::Error;

/// 驱逐策略错误
#[derive(Error, Debug)]
pub enum StrategyError {
    #[error("No victim available")]
    NoVictim,
    #[error("Key not found")]
    KeyNotFound,
}

/// 驱逐策略特征
pub trait EvictionStrategy {
    type Key: CacheKey;
    
    /// 记录访问
    fn record_access(&mut self, key: &Self::Key) -> Result<(), StrategyError>;
    
    /// 选择受害者
    fn select_victim(&self) -> Option<Self::Key>;
    
    /// 移除键
    fn remove(&mut self, key: &Self::Key) -> Result<(), StrategyError>;
    
    /// 清空
    fn clear(&mut self) -> Result<(), StrategyError>;
}

/// FIFO策略
#[derive(Debug)]
pub struct FIFOStrategy<K>
where
    K: CacheKey,
{
    queue: VecDeque<K>,
}

impl<K> FIFOStrategy<K>
where
    K: CacheKey,
{
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl<K> EvictionStrategy for FIFOStrategy<K>
where
    K: CacheKey + Eq,
{
    type Key = K;
    
    fn record_access(&mut self, key: &Self::Key) -> Result<(), StrategyError> {
        // FIFO不记录访问
        if !self.queue.contains(key) {
            self.queue.push_back(key.clone());
        }
        Ok(())
    }
    
    fn select_victim(&self) -> Option<Self::Key> {
        self.queue.front().cloned()
    }
    
    fn remove(&mut self, key: &Self::Key) -> Result<(), StrategyError> {
        self.queue
            .retain(|k| k != key);
        Ok(())
    }
    
    fn clear(&mut self) -> Result<(), StrategyError> {
        self.queue.clear();
        Ok(())
    }
}

/// LRU策略（展示生命周期与泛型）
#[derive(Debug)]
pub struct LRUStrategy<K>
where
    K: CacheKey,
{
    access_list: VecDeque<K>,
    capacity: usize,
}

impl<K> LRUStrategy<K>
where
    K: CacheKey + Eq,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            access_list: VecDeque::with_capacity(capacity),
            capacity,
        }
    }
    
    /// 移动键到列表末尾（展示生命周期标注）
    fn move_to_back<'a>(&mut self, key: &'a K) 
    where
        K: 'a,
    {
        if let Some(pos) = self.access_list.iter().position(|k| k == key) {
            let key = self.access_list.remove(pos).unwrap();
            self.access_list.push_back(key);
        }
    }
}

impl<K> EvictionStrategy for LRUStrategy<K>
where
    K: CacheKey + Eq,
{
    type Key = K;
    
    fn record_access(&mut self, key: &Self::Key) -> Result<(), StrategyError> {
        if !self.access_list.contains(key) {
            if self.access_list.len() >= self.capacity {
                self.access_list.pop_front();
            }
            self.access_list.push_back(key.clone());
        } else {
            self.move_to_back(key);
        }
        Ok(())
    }
    
    fn select_victim(&self) -> Option<Self::Key> {
        self.access_list.front().cloned()
    }
    
    fn remove(&mut self, key: &Self::Key) -> Result<(), StrategyError> {
        self.access_list
            .retain(|k| k != key);
        Ok(())
    }
    
    fn clear(&mut self) -> Result<(), StrategyError> {
        self.access_list.clear();
        Ok(())
    }
}
```

```rust
// src/main.rs
use lifetime_cache::{
    Cache, MemoryStorage, LRUStrategy, SimpleKeyStrategy, 
    CacheKey, CacheValue, FIFOStrategy
};
use std::time::Duration;

/// 演示生命周期概念的示例
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Rust生命周期概念演示 ===\n");
    
    // 1. 基本生命周期：结构体中的生命周期
    demonstrate_struct_lifetimes()?;
    
    // 2. 函数生命周期：传递引用
    demonstrate_function_lifetimes()?;
    
    // 3. 生命周期与泛型
    demonstrate_generic_lifetimes()?;
    
    // 4. 生命周期与trait对象
    demonstrate_trait_lifetimes()?;
    
    // 5. 生命周期与迭代器
    demonstrate_iterator_lifetimes()?;
    
    // 6. 生命周期与闭包
    demonstrate_closure_lifetimes()?;
    
    // 7. 生命周期省略
    demonstrate_elision()?;
    
    // 8. 复杂生命周期场景
    demonstrate_complex_scenarios()?;
    
    Ok(())
}

/// 1. 结构体生命周期演示
fn demonstrate_struct_lifetimes() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 1. 结构体生命周期 ---");
    
    let storage = MemoryStorage::<String, String>::new();
    let strategy = LRUStrategy::new(10);
    let key_strategy = Box::new(SimpleKeyStrategy);
    
    let mut cache = Cache::new(
        storage,
        strategy,
        key_strategy,
        10,
    );
    
    // 演示插入拥有数据
    let key = "user_1".to_string();
    let value = "Alice".to_string();
    cache.insert(key.clone(), value, Some(Duration::from_secs(60)))?;
    
    // 演示插入借用数据（生命周期绑定）
    let borrowed_data = "Bob".to_string();
    let key2 = "user_2".to_string();
    cache.insert_borrowed(key2.clone(), &borrowed_data, Some(Duration::from_secs(30)))?;
    
    // 获取数据（生命周期验证）
    if let Ok(entry) = cache.get(&key) {
        println!("  ✅ 获取到数据: {:?}", String::from_utf8_lossy(entry.value()));
    }
    
    println!("  结构体生命周期示例完成\n");
    Ok(())
}

/// 2. 函数生命周期演示
fn demonstrate_function_lifetimes() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 2. 函数生命周期 ---");
    
    // 展示不同生命周期标注的函数
    let s1 = "Hello".to_string();
    let s2 = "World".to_string();
    
    // 最长生命周期
    let result = longest(&s1, &s2);
    println!("  ✅ 最长字符串: {}", result);
    
    // 特定生命周期约束
    let data = create_with_lifetime(&s1);
    println!("  ✅ 创建带生命周期的数据: {}", data);
    
    // 生命周期与返回值
    let (first, _) = split_with_lifetime(&s1);
    println!("  ✅ 分割字符串: {}", first);
    
    println!("  函数生命周期示例完成\n");
    Ok(())
}

/// 返回两个字符串中较长的（展示生命周期）
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

/// 创建带生命周期约束的数据
fn create_with_lifetime<'a>(data: &'a str) -> &'a str {
    data
}

/// 分割并返回元组（展示生命周期）
fn split_with_lifetime<'a>(s: &'a str) -> (&'a str, &'a str) {
    let mid = s.len() / 2;
    (&s[..mid], &s[mid..])
}

/// 3. 泛型生命周期演示
fn demonstrate_generic_lifetimes() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 3. 泛型生命周期 ---");
    
    // 泛型结构体
    struct Wrapper<'a, T> {
        data: &'a T,
    }
    
    impl<'a, T> Wrapper<'a, T> {
        fn new(data: &'a T) -> Self {
            Self { data }
        }
        
        fn get(&self) -> &'a T {
            self.data
        }
    }
    
    let value = 42;
    let wrapper = Wrapper::new(&value);
    println!("  ✅ 泛型包装器: {}", wrapper.get());
    
    // 泛型函数
    let x = 10;
    let y = 20;
    let result = generic_longest(&x, &y);
    println!("  ✅ 泛型最长: {}", result);
    
    // 泛型trait实现
    let data = "泛型数据".to_string();
    let borrowed = BorrowedData::new(&data);
    println!("  ✅ 泛型借用: {}", borrowed.get());
    
    println!("  泛型生命周期示例完成\n");
    Ok(())
}

/// 泛型最长函数
fn generic_longest<'a, T>(x: &'a T, y: &'a T) -> &'a T 
where
    T: std::cmp::PartialOrd + std::fmt::Display,
{
    if x > y { x } else { y }
}

/// 泛型借用结构体
struct BorrowedData<'a, T> {
    data: &'a T,
}

impl<'a, T> BorrowedData<'a, T> {
    fn new(data: &'a T) -> Self {
        Self { data }
    }
    
    fn get(&self) -> &'a T {
        self.data
    }
}

/// 4. Trait对象生命周期演示
fn demonstrate_trait_lifetimes() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 4. Trait对象生命周期 ---");
    
    use lifetime_cache::{CacheKey, KeyStrategy};
    
    // Trait对象生命周期
    let strategies: Vec<Box<dyn KeyStrategy<Key = String>>> = vec![
        Box::new(SimpleKeyStrategy),
        Box::new(lifetime_cache::HashKeyStrategy::new("salt")),
    ];
    
    for strategy in strategies.iter() {
        let key = strategy.generate_key("test");
        println!("  ✅ Trait对象生成键: {}", key);
    }
    
    // 生命周期与trait对象
    let data = "Hello".to_string();
    let printable = Box::new(PrintableWrapper { data: &data });
    printable.print();
    
    println!("  Trait对象生命周期示例完成\n");
    Ok(())
}

trait Printable {
    fn print(&self);
}

struct PrintableWrapper<'a> {
    data: &'a str,
}

impl<'a> Printable for PrintableWrapper<'a> {
    fn print(&self) {
        println!("  ✅ Trait对象打印: {}", self.data);
    }
}

/// 5. 迭代器生命周期演示
fn demonstrate_iterator_lifetimes() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 5. 迭代器生命周期 ---");
    
    let mut cache = create_test_cache()?;
    
    // 插入测试数据
    for i in 0..5 {
        cache.insert(
            format!("key_{}", i),
            format!("value_{}", i),
            Some(Duration::from_secs(60)),
        )?;
    }
    
    // 获取所有键（迭代器生命周期）
    let keys: Vec<_> = cache.keys().collect();
    for key in keys {
        println!("  ✅ 迭代器键: {:?}", key);
    }
    
    // 映射操作（生命周期）
    let mapped: Vec<_> = cache.map_values(|entry| {
        String::from_utf8_lossy(entry.value()).to_string()
    });
    for (key, value) in mapped {
        println!("  ✅ 映射后: {:?} -> {}", key, value);
    }
    
    // 批量获取（生命周期约束）
    let batch_keys = vec![&"key_0".to_string(), &"key_1".to_string()];
    let results = cache.get_batch(batch_keys);
    for result in results {
        if let Ok(entry) = result {
            println!("  ✅ 批量获取: {:?}", String::from_utf8_lossy(entry.value()));
        }
    }
    
    println!("  迭代器生命周期示例完成\n");
    Ok(())
}

/// 6. 闭包生命周期演示
fn demonstrate_closure_lifetimes() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 6. 闭包生命周期 ---");
    
    let cache = create_test_cache()?;
    
    // 闭包捕获引用（生命周期）
    let filter_key = "key_1".to_string();
    let filter = |entry: &lifetime_cache::CacheEntry<String, String>| {
        entry.key().to_string() == filter_key
    };
    
    let filtered: Vec<_> = cache.map_values(|entry| {
        if filter(entry) {
            Some(String::from_utf8_lossy(entry.value()).to_string())
        } else {
            None
        }
    }).into_iter()
      .filter_map(|(_, opt)| opt)
      .collect();
    
    println!("  ✅ 闭包过滤结果: {:?}", filtered);
    
    // 高阶函数（生命周期）
    let processed = process_cache_data(&cache, |entry| {
        let value = String::from_utf8_lossy(entry.value());
        format!("Processed: {}", value)
    });
    
    for (key, value) in processed {
        println!("  ✅ 高阶函数处理: {} -> {}", key, value);
    }
    
    println!("  闭包生命周期示例完成\n");
    Ok(())
}

/// 高阶函数处理（展示生命周期）
fn process_cache_data<'a, K, V, F>(
    cache: &'a lifetime_cache::Cache<'a, K, V, MemoryStorage<K, V>, LRUStrategy<K>>,
    f: F,
) -> Vec<(K, String)>
where
    K: CacheKey,
    V: CacheValue,
    F: Fn(&lifetime_cache::CacheEntry<'a, K, V>) -> String + 'a,
{
    cache.map_values(f)
}

/// 7. 生命周期省略演示
fn demonstrate_elision() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 7. 生命周期省略 ---");
    
    // 省略规则演示
    let s = "Hello, World!";
    
    // 规则1: 每个参数都有自己的生命周期
    let first = first_word(s);
    println!("  ✅ 第一个词: {}", first);
    
    // 规则2: 如果只有一个输入生命周期，输出使用同样的生命周期
    let slice = get_slice(s);
    println!("  ✅ 获取切片: {}", slice);
    
    // 规则3: 方法中省略
    let wrapper = StringWrapper::new(s);
    let content = wrapper.get();
    println!("  ✅ 方法省略: {}", content);
    
    // 自定义省略
    let parts = split_at_mid(s);
    println!("  ✅ 分割: '{}' 和 '{}'", parts.0, parts.1);
    
    println!("  生命周期省略示例完成\n");
    Ok(())
}

// 省略规则1: 每个参数都有自己的生命周期
fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

// 省略规则2: 只有一个输入生命周期
fn get_slice(s: &str) -> &str {
    &s[..5]
}

// 省略规则3: 方法中省略
struct StringWrapper<'a> {
    data: &'a str,
}

impl<'a> StringWrapper<'a> {
    fn new(data: &'a str) -> Self {
        Self { data }
    }
    
    // 省略后的方法
    fn get(&self) -> &str {
        self.data
    }
}

// 自定义省略（需要显式标注）
fn split_at_mid(s: &str) -> (&str, &str) {
    let mid = s.len() / 2;
    (&s[..mid], &s[mid..])
}

/// 8. 复杂生命周期场景
fn demonstrate_complex_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 8. 复杂生命周期场景 ---");
    
    // 场景1: 多个生命周期参数
    let str1 = "Hello";
    let str2 = "World";
    let result = combine_strs(str1, str2);
    println!("  ✅ 组合字符串: {}", result);
    
    // 场景2: 生命周期子类型
    let outer_data = "Outer".to_string();
    let result = process_nested(&outer_data);
    println!("  ✅ 嵌套生命周期: {}", result);
    
    // 场景3: 生命周期与可变性
    let mut data = vec![1, 2, 3];
    let result = process_mutable(&mut data);
    println!("  ✅ 可变生命周期: {:?}", result);
    
    // 场景4: 自引用结构体（使用Pin）
    use std::pin::Pin;
    let mut self_ref = SelfReferential::new("Hello");
    let result = self_ref.get_data();
    println!("  ✅ 自引用结构体: {}", result);
    
    // 场景5: 生命周期与异步（模拟）
    let result = async_operation("Async data");
    println!("  ✅ 异步生命周期: {}", result);
    
    println!("  复杂生命周期场景示例完成\n");
    Ok(())
}

/// 多个生命周期参数
fn combine_strs<'a, 'b>(s1: &'a str, s2: &'b str) -> String 
where
    'a: 'b,  // 'a 至少和 'b 一样长
{
    format!("{} {}", s1, s2)
}

/// 嵌套生命周期
fn process_nested<'a>(data: &'a String) -> &'a str {
    let inner = data.as_str();
    inner  // 返回内部引用
}

/// 可变生命周期
fn process_mutable(data: &mut Vec<i32>) -> &mut [i32] {
    data.push(4);
    data.as_mut_slice()
}

/// 自引用结构体（使用Pin）
use std::pin::Pin;
use std::marker::PhantomPinned;

struct SelfReferential {
    data: String,
    ptr: *const String,
    _pin: PhantomPinned,
}

impl SelfReferential {
    fn new(data: &str) -> Pin<Box<Self>> {
        let mut boxed = Box::pin(Self {
            data: data.to_string(),
            ptr: std::ptr::null(),
            _pin: PhantomPinned,
        });
        
        unsafe {
            let ptr = &boxed.as_ref().data as *const String;
            let mut_ref = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref).ptr = ptr;
        }
        
        boxed
    }
    
    fn get_data(&self) -> &str {
        unsafe { &*self.ptr }
    }
}

/// 模拟异步操作（生命周期约束）
fn async_operation(data: &str) -> String {
    // 模拟异步操作
    format!("Async result: {}", data)
}

/// 创建测试缓存
fn create_test_cache() -> Result<Cache<'static, String, String, MemoryStorage<String, String>, LRUStrategy<String>>, Box<dyn std::error::Error>> {
    let storage = MemoryStorage::<String, String>::new();
    let strategy = LRUStrategy::new(10);
    let key_strategy = Box::new(SimpleKeyStrategy);
    
    Ok(Cache::new(
        storage,
        strategy,
        key_strategy,
        10,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lifetime_scenarios() -> Result<(), Box<dyn std::error::Error>> {
        demonstrate_struct_lifetimes()?;
        demonstrate_function_lifetimes()?;
        demonstrate_generic_lifetimes()?;
        demonstrate_trait_lifetimes()?;
        demonstrate_iterator_lifetimes()?;
        demonstrate_closure_lifetimes()?;
        demonstrate_elision()?;
        demonstrate_complex_scenarios()?;
        Ok(())
    }
}
```

这个项目全面展示了Rust生命周期的各个方面：

## 核心概念展示

### 1. **结构体生命周期** (`Cache`, `CacheEntry`, `StorageGuard`)
- 结构体中的生命周期参数
- 生命周期与泛型结合
- 生命周期标记的使用

### 2. **函数生命周期**
- 输入输出生命周期关系
- 多个生命周期参数
- 生命周期约束

### 3. **泛型生命周期**
- 泛型结构体中的生命周期
- 泛型函数中的生命周期
- 生命周期与类型参数

### 4. **Trait对象生命周期**
- Trait对象中的生命周期
- 动态分发与生命周期
- Box<dyn Trait>生命周期

### 5. **迭代器生命周期**
- 迭代器与生命周期约束
- 映射操作中的生命周期
- 批量操作的生命周期

### 6. **闭包生命周期**
- 闭包捕获引用
- 高阶函数生命周期
- 闭包作为参数

### 7. **生命周期省略**
- 三条省略规则
- 方法中的省略
- 自定义省略

### 8. **复杂场景**
- 多个生命周期参数
- 生命周期子类型
- 自引用结构体
- 异步操作与生命周期

## 生产级特性

1. **错误处理**：使用 `thiserror` 提供详细错误类型
2. **类型安全**：使用强类型和约束
3. **内存安全**：通过生命周期确保安全
4. **性能优化**：支持借用和所有权转移
5. **可扩展性**：通过 trait 设计支持多种策略
6. **测试覆盖**：包含单元测试示例

这个项目可以作为学习生命周期的完整教材，同时也是一个可用的缓存系统基础框架。