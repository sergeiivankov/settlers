use sea_orm::{
  ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey,
  DeriveRelation, EntityTrait, EnumIter, PrimaryKeyTrait
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false, column_type = "Char(Some(64))")]
  pub token: String,
  #[sea_orm(indexed)]
  pub user_id: u32,
  pub expires: u64
}

#[derive(Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}