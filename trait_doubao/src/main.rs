use std::fmt;
use std::fs;
use std::path::Path;

// ======================================================================
// 1. 基础 trait：数据校验特征（基础能力，无关联类型）
// ======================================================================
/// 所有实体都需要实现合法性校验
pub trait Validatable {
    /// 校验自身数据，非法返回错误信息
    fn validate(&self) -> Result<(), String>;

    // 默认实现：快速判断是否合法
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

// ======================================================================
// 2. 核心 trait：可持久化对象【重点！包含关联类型 + trait继承】
// ======================================================================
/// 持久化实体特征
/// Supertrait：要求实现者必须同时实现 Debug + Validatable
pub trait Persistable: fmt::Debug + Validatable {
    // 关联类型：每个实现类型自定义唯一ID类型（优于泛型，语义更清晰）
    type Id: Clone + fmt::Display + Eq;

    /// 获取主键ID
    fn id(&self) -> &Self::Id;

    /// 序列化为字符串（用于落盘）
    fn serialize(&self) -> Result<String, String>;

    /// 从字符串反序列化
    fn deserialize(raw: &str) -> Result<Self, String>
    where
        Self: Sized; // deserialize 构造实例，要求类型大小已知，不能用于dyn trait对象
}

// ======================================================================
// 3. 存储后端 trait（抽象存储行为，用于动态分发 dyn）
// ======================================================================
/// 通用存储后端，支持增查
pub trait StorageBackend {
    /// 根据ID加载持久化对象
    fn load<T: Persistable>(&self, id: &T::Id) -> Result<T, String>;

    /// 保存持久化对象
    fn save<T: Persistable>(&mut self, entity: &T) -> Result<(), String>;
}

// ======================================================================
// 4. 业务实体：User、Order 实现 Persistable + Validatable
// ======================================================================
#[derive(Debug, Clone)]
pub struct User {
    user_id: u64,
    username: String,
    age: u8,
}

impl Validatable for User {
    fn validate(&self) -> Result<(), String> {
        if self.username.is_empty() {
            return Err("用户名不能为空".to_string());
        }
        if self.age > 120 {
            return Err("年龄不合法".to_string());
        }
        Ok(())
    }
}

impl Persistable for User {
    type Id = u64;

    fn id(&self) -> &Self::Id {
        &self.user_id
    }

    fn serialize(&self) -> Result<String, String> {
        // 生产环境替换为serde json
        Ok(format!(
            "user|{}|{}|{}",
            self.user_id, self.username, self.age
        ))
    }

    fn deserialize(raw: &str) -> Result<Self, String> {
        let parts: Vec<&str> = raw.split('|').collect();
        if parts.len() != 4 || parts[0] != "user" {
            return Err("用户数据格式错误".to_string());
        }
        let user_id = parts[1]
            .parse()
            .map_err(|e| format!("id解析失败:{}", e))?;
        let username = parts[2].to_string();
        let age = parts[3]
            .parse()
            .map_err(|e| format!("age解析失败:{}", e))?;

        Ok(User {
            user_id,
            username,
            age,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Order {
    order_id: String,
    user_id: u64,
    amount: f64,
}

impl Validatable for Order {
    fn validate(&self) -> Result<(), String> {
        if self.order_id.is_empty() {
            return Err("订单号不能为空".to_string());
        }
        if self.amount < 0.0 {
            return Err("金额不能为负数".to_string());
        }
        Ok(())
    }
}

impl Persistable for Order {
    type Id = String;

    fn id(&self) -> &Self::Id {
        &self.order_id
    }

    fn serialize(&self) -> Result<String, String> {
        Ok(format!(
            "order|{}|{}|{}",
            self.order_id, self.user_id, self.amount
        ))
    }

    fn deserialize(raw: &str) -> Result<Self, String> {
        let parts: Vec<&str> = raw.split('|').collect();
        if parts.len() != 4 || parts[0] != "order" {
            return Err("订单数据格式错误".to_string());
        }
        let order_id = parts[1].to_string();
        let user_id = parts[2]
            .parse()
            .map_err(|e| format!("user_id解析失败:{}", e))?;
        let amount = parts[3]
            .parse()
            .map_err(|e| format!("amount解析失败:{}", e))?;

        Ok(Order {
            order_id,
            user_id,
            amount,
        })
    }
}

// ======================================================================
// 5. 实现两种存储后端：文件存储 + 内存缓存存储
// ======================================================================
/// 文件持久化后端
#[derive(Debug)]
pub struct FileStorage {
    base_dir: String,
}

impl FileStorage {
    pub fn new(base_dir: &str) -> Self {
        // 创建目录
        let _ = fs::create_dir_all(base_dir);
        Self {
            base_dir: base_dir.to_string(),
        }
    }

    fn entity_path<T: Persistable>(&self, id: &T::Id) -> String {
        format!("{}/{}.dat", self.base_dir, id)
    }
}

impl StorageBackend for FileStorage {
    fn load<T: Persistable>(&self, id: &T::Id) -> Result<T, String> {
        let path = self.entity_path::<T>(id);
        let raw = fs::read_to_string(path).map_err(|e| format!("读取文件失败:{}", e))?;
        T::deserialize(&raw)
    }

    fn save<T: Persistable>(&mut self, entity: &T) -> Result<(), String> {
        entity.validate()?; // 保存前强制校验
        let raw = entity.serialize()?;
        let path = self.entity_path::<T>(entity.id());
        fs::write(path, raw).map_err(|e| format!("写入文件失败:{}", e))
    }
}

/// 内存缓存后端
use std::collections::HashMap;
#[derive(Debug, Default)]
pub struct MemStorage {
    cache: HashMap<String, String>,
}

impl StorageBackend for MemStorage {
    fn load<T: Persistable>(&self, id: &T::Id) -> Result<T, String> {
        let key = id.to_string();
        let raw = self
            .cache
            .get(&key)
            .ok_or_else(|| format!("id={} 不存在", id))?;
        T::deserialize(raw)
    }

    fn save<T: Persistable>(&mut self, entity: &T) -> Result<(), String> {
        entity.validate()?;
        let raw = entity.serialize()?;
        let key = entity.id().to_string();
        self.cache.insert(key, raw);
        Ok(())
    }
}

// ======================================================================
// 6. 工具泛型函数：使用 trait bound（静态分发）
// ======================================================================
/// 通用业务函数：打印实体信息
pub fn print_entity<T: Persistable>(entity: &T) {
    println!("实体ID: {}", entity.id());
    match entity.serialize() {
        Ok(data) => println!("序列化数据: {}", data),
        Err(e) => println!("序列化失败:{}", e),
    }
    if entity.is_valid() {
        println!("✅ 数据合法");
    } else {
        println!("❌ 数据非法");
    }
}

// 带 where 语法的版本（复杂约束推荐where）
pub fn check_entity<T>(entity: &T) -> bool
where
    T: Persistable,
{
    entity.is_valid()
}

// ======================================================================
// 7. 动态分发 dyn StorageBackend（运行时选择存储引擎）
// ======================================================================
fn run_storage_ops(backend: &mut dyn StorageBackend) -> Result<(), String> {
    // 创建用户
    let user = User {
        user_id: 1001,
        username: "zhangsan".to_string(),
        age: 25,
    };
    backend.save(&user)?;
    let loaded_user: User = backend.load(&1001)?;
    println!("加载用户：{:?}", loaded_user);

    // 创建订单
    let order = Order {
        order_id: "ORD20260729001".to_string(),
        user_id: 1001,
        amount: 99.5,
    };
    backend.save(&order)?;
    let loaded_order: Order = backend.load(&"ORD20260729001".to_string())?;
    println!("加载订单：{:?}", loaded_order);

    Ok(())
}

// ======================================================================
// 8. main 入口运行测试
// ======================================================================
fn main() -> Result<(), String> {
    println!("===== 使用内存存储 =====");
    let mut mem_store = MemStorage::default();
    run_storage_ops(&mut mem_store)?;

    println!("\n===== 使用文件存储（写入 ./data 目录） =====");
    let mut file_store = FileStorage::new("./data");
    run_storage_ops(&mut file_store)?;

    // 单独测试泛型工具函数
    let test_user = User {
        user_id: 9999,
        username: "lisi".to_string(),
        age: 30,
    };
    print_entity(&test_user);

    // 非法数据测试校验
    let bad_user = User {
        user_id: 8888,
        username: "".to_string(),
        age: 200,
    };
    println!("非法用户校验结果：{}", bad_user.validate().unwrap_err());

    Ok(())
}
