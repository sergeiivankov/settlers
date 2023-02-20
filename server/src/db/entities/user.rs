use sea_orm::{
  ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, EntityTrait, EnumIter,
  PrimaryKeyTrait
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: u32,
  #[sea_orm(unique, column_type = "String(Some(32))")]
  pub name: String,
  #[sea_orm(column_type = "Char(Some(32))")]
  pub photo: String,
  pub tag: u16
}

#[derive(Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(
    belongs_to = "super::auth::Entity",
    from = "Column::Id",
    to = "super::auth::Column::UserId"
  )]
  Auth,
  #[sea_orm(
    belongs_to = "super::session::Entity",
    from = "Column::Id",
    to = "super::session::Column::UserId"
  )]
  Session
}

impl ActiveModelBehavior for ActiveModel {}