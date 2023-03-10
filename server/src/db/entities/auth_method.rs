use sea_orm::{
  ActiveModelBehavior, DeriveActiveEnum, DeriveEntityModel, DerivePrimaryKey,
  DeriveRelation, EntityTrait, EnumIter, PrimaryKeyTrait
};

// Eq trait not required for used in DeriveEntityModel enums
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "u8", db_type = "TinyUnsigned")]
pub enum Method {
  Password = 0,
  Email = 1,
  Telegram = 2
}

#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "auths_methods")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub user_id: u32,
  #[sea_orm(primary_key)]
  pub method: Method,
  #[sea_orm(column_type = "String(Some(256))")]
  pub data: String
}

#[derive(Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}