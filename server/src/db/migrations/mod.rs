mod m0001_create_users_table;
mod m0002_create_sessions_table;

use sea_orm::{ schema::Schema, EntityTrait };
use sea_orm_migration::{
  async_trait::async_trait, manager::SchemaManager, migrator::MigratorTrait, DbErr, MigrationTrait
};
use sea_query::{ index::IndexCreateStatement, table::{ TableCreateStatement, TableDropStatement } };

type MigrationResult = Result<(), DbErr>;

pub struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
      Box::new(m0001_create_users_table::Migration),
      Box::new(m0002_create_sessions_table::Migration)
    ]
  }
}

fn create_statements<E>(
  manager: &SchemaManager, entity: E
) -> (TableCreateStatement, Vec<IndexCreateStatement>)
where
  E: EntityTrait
{
  let builder = manager.get_database_backend();
  let schema = Schema::new(builder);

  let table_create_stmt = schema.create_table_from_entity(entity);
  let index_create_stmts = schema.create_index_from_entity(entity);

  (table_create_stmt, index_create_stmts)
}

async fn simple_up<E>(manager: &SchemaManager<'_>, entity: E) -> MigrationResult
where
  E: EntityTrait
{
  let mut stmts = create_statements(manager, entity);

  manager.create_table(stmts.0.if_not_exists().to_owned()).await?;
  for index_create_stmt in stmts.1 {
    manager.create_index(index_create_stmt).await?;
  }

  Ok(())
}

async fn simple_down<E>(manager: &SchemaManager<'_>, entity: E) -> MigrationResult
where
  E: EntityTrait
{
  let table_create_stmt = create_statements(manager, entity).0;

  let table_name = match table_create_stmt.get_table_name() {
    Some(table_name) => table_name,
    None => return Err(DbErr::Custom(String::from(
      "Get table name from TableCreateStatement error"
    )))
  }.to_owned();

  let table_drop_stmt = TableDropStatement::new()
    .table(table_name)
    .cascade()
    .if_exists()
    .to_owned();

  manager.drop_table(table_drop_stmt).await
}