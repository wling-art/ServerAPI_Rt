use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "ban_type_enum")]
pub enum BanTypeEnum {
    #[sea_orm(string_value = "mute")]
    Mute,
    #[sea_orm(string_value = "ban")]
    Ban,
    #[sea_orm(string_value = "temp_ban")]
    TempBan,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "ban_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub ban_type: BanTypeEnum,
    #[sea_orm(column_type = "Text")]
    pub reason: Option<String>,
    #[schema(value_type = String, format = DateTime)]
    pub started_at: ChronoDateTimeUtc,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub ended_at: Option<ChronoDateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
