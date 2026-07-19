// 数据库层：SQLite 连接 + Repository 模式
// =========================================
// Db 是全局数据库句柄，内部持有 SqlitePool。
// 通过 Db::open(path) 打开文件数据库，Db::open_in_memory() 打开内存库（测试用）。
//
// Repository 模式：每个实体有自己的 Repo struct，通过 Db 的工厂方法获取。
//   db.characters() → CharacterRepo
//   db.scenes()     → SceneRepo
//   db.memories()   → MemoryRepo
//   db.sensations() → SensationRepo
//   db.derivations()-> DerivationRepo（组合 Repo，跨表事务）

mod character_repo; // 角色 CRUD（包含性格标签 & 技能标签子表）
mod derivation_repo; // 推导结果写入（SensorySelection + CharacterMemory 跨表事务）
mod memory_repo; // 角色记忆查询 & 插入
mod scene_repo; // 场景 CRUD & 参与者关系
mod schema; // DDL 定义 & 迁移
mod sensation_repo; // 五感数据查询 & 插入

pub use character_repo::CharacterRepo;
pub use derivation_repo::DerivationRepo;
pub use memory_repo::MemoryRepo;
pub use scene_repo::SceneRepo;
pub use schema::migrate;
pub use sensation_repo::SensationRepo;

use crate::models::StoryError;
use sqlx::sqlite::SqlitePool;
use std::str::FromStr;

/// 数据库句柄
/// ============
/// 封装 SqlitePool，提供各 Repository 的工厂方法。
/// Clone 是廉价的（Arc 内部引用计数）。
#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// 打开文件数据库（自动创建 + 迁移）
    ///
    /// # 参数
    /// - `path` — SQLite 连接字符串，如 "novels.db"
    ///
    /// 行为：
    ///   - 文件不存在时自动创建（create_if_missing）
    ///   - 启用外键约束（foreign_keys）
    ///   - 自动执行 schema 迁移建表
    pub async fn open(path: &str) -> Result<Self, StoryError> {
        let opts = sqlx::sqlite::SqliteConnectOptions::from_str(path)?
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePool::connect_with(opts).await?;
        migrate(&pool).await?;
        Ok(Self { pool })
    }

    /// 打开内存数据库（测试用）
    pub async fn open_in_memory() -> Result<Self, StoryError> {
        let opts =
            sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePool::connect_with(opts).await?;
        migrate(&pool).await?;
        Ok(Self { pool })
    }

    /// 获取底层连接池引用
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ---- Repository 工厂方法 ----
    pub fn characters(&self) -> CharacterRepo {
        CharacterRepo::new(self.pool.clone())
    }
    pub fn scenes(&self) -> SceneRepo {
        SceneRepo::new(self.pool.clone())
    }
    pub fn memories(&self) -> MemoryRepo {
        MemoryRepo::new(self.pool.clone())
    }
    pub fn sensations(&self) -> SensationRepo {
        SensationRepo::new(self.pool.clone())
    }
    pub fn derivations(&self) -> DerivationRepo {
        DerivationRepo::new(self.pool.clone())
    }
}
