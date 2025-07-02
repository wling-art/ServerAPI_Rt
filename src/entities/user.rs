use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "role_enum")]
pub enum RoleEnum {
    #[sea_orm(string_value = "user")]
    User,
    #[sea_orm(string_value = "admin")]
    Admin,
}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "ser_role_enum")]
pub enum SerRoleEnum {
    #[sea_orm(string_value = "owner")]
    Owner,
    #[sea_orm(string_value = "admin")]
    Admin,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub username: String,
    #[sea_orm(unique)]
    pub email: String,
    pub display_name: String,
    pub hashed_password: String,
    pub avatar_hash_id: Option<String>,
    pub role: RoleEnum,
    pub is_active: bool,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: ChronoDateTimeUtc,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_login: Option<ChronoDateTimeUtc>,
    pub last_login_ip: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::file::Entity",
        from = "Column::AvatarHashId",
        to = "super::file::Column::HashValue",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    AvatarFile,
    #[sea_orm(has_many = "super::ticket::Entity")]
    CreatedTickets,
    #[sea_orm(has_many = "super::ban_record::Entity")]
    BanRecords,
}

impl Related<super::file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AvatarFile.def()
    }
}

impl Related<super::ticket::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedTickets.def()
    }
}

impl Related<super::ban_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BanRecords.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
