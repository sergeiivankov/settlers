use sea_orm::DeriveMigrationName;
use sea_orm_migration::{ async_trait::async_trait, manager::SchemaManager, MigrationTrait };
use super::{ MigrationResult, structure_from_entity };
use crate::db::entities::{
  auth_method::Entity as AuthMethod,
  auth_session::Entity as AuthSession,
  user::Entity as User
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> MigrationResult {
    structure_from_entity(manager, AuthMethod).await?;
    structure_from_entity(manager, AuthSession).await?;
    structure_from_entity(manager, User).await?;

    Ok(())
  }
}