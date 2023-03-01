mod m0001_initial_structure;

use sea_orm::{ schema::Schema, EntityTrait };
use sea_orm_migration::{
  async_trait::async_trait, manager::SchemaManager, migrator::MigratorTrait, DbErr, MigrationTrait
};
use sea_query::{ index::IndexCreateStatement, table::TableCreateStatement };

type MigrationResult = Result<(), DbErr>;

pub struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m0001_initial_structure::Migration)]
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

// TODO: for first stable release remove and rewrite migrations to manual creation using statements
//       to later creation modifying base tables migrations and changing entities
async fn structure_from_entity<E>(manager: &SchemaManager<'_>, entity: E) -> MigrationResult
where
  E: EntityTrait
{
  let mut stmts = create_statements(manager, entity);

  manager.create_table(stmts.0.if_not_exists().clone()).await?;
  for index_create_stmt in stmts.1 {
    manager.create_index(index_create_stmt).await?;
  }

  Ok(())
}