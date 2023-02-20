use sea_orm::{
  ActiveModelBehavior, ColumnTypeTrait, DeriveActiveEnum, DeriveEntityModel, DerivePrimaryKey,
  DeriveRelation, EntityTrait, EnumIter, PrimaryKeyTrait
};

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "u8", db_type = "TinyUnsigned")]
pub enum Kind {
  Password = 0,
  Email = 1,
  Telegram = 2
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "auths")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub user_id: u32,
  #[sea_orm(primary_key)]
  pub kind: Kind,
  #[sea_orm(column_type = "String(Some(256))")]
  pub relation: String
}

#[derive(Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}