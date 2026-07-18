mod character_repo;
mod derivation_repo;
mod memory_repo;
mod schema;
mod scene_repo;
mod sensation_repo;

pub use character_repo::CharacterRepo;
pub use derivation_repo::DerivationRepo;
pub use memory_repo::MemoryRepo;
pub use schema::migrate;
pub use scene_repo::SceneRepo;
pub use sensation_repo::SensationRepo;

use sqlx::sqlite::SqlitePool;
use std::str::FromStr;
use crate::models::StoryError;

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn open(path: &str) -> Result<Self, StoryError> {
        let opts = sqlx::sqlite::SqliteConnectOptions::from_str(path)?
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePool::connect_with(opts).await?;
        migrate(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn open_in_memory() -> Result<Self, StoryError> {
        let opts = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")?
            .foreign_keys(true);
        let pool = SqlitePool::connect_with(opts).await?;
        migrate(&pool).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

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
