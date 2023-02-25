use sea_orm::DeriveMigrationName;
use sea_orm_migration::{ async_trait::async_trait, manager::SchemaManager, MigrationTrait };
use super::{ MigrationResult, up_from_entity };
use crate::db::entities::user::Entity;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> MigrationResult {
    up_from_entity(manager, Entity).await
  }
}