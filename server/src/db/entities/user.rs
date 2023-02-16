use sea_orm::{
  ActiveModelBehavior, ColumnTypeTrait, DeriveActiveEnum, DeriveEntityModel, DerivePrimaryKey,
  DeriveRelation, EntityTrait, EnumIter, PrimaryKeyTrait
};

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "u8", db_type = "TinyUnsigned")]
pub enum RelationType {
  Email = 0,
  Telegram = 1
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: u64,
  pub relation_type: RelationType,
  #[sea_orm(unique)]
  pub relation: String,
  pub name: String,
  pub photo: String
}

#[derive(Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}