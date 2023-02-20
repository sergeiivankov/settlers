use sea_orm::DeriveMigrationName;
use sea_orm_migration::{ async_trait::async_trait, manager::SchemaManager, MigrationTrait };
use super::{ MigrationResult, simple_up, simple_down };
use crate::db::entities::auth_method::Entity;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> MigrationResult {
    simple_up(manager, Entity).await
  }

  async fn down(&self, manager: &SchemaManager) -> MigrationResult {
    simple_down(manager, Entity).await
  }
}