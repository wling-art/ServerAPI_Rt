use chrono::{DateTime, Utc};
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
    #[sea_orm(string_value = "moderator")]
    Moderator,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
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
    pub role: RoleEnum,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub last_login_ip: Option<String>,
    pub avatar_hash_id: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::ban_records::Entity")]
    BanRecords,
    #[sea_orm(
        belongs_to = "super::files::Entity",
        from = "Column::AvatarHashId",
        to = "super::files::Column::HashValue",
        on_update = "Restrict",
        on_delete = "SetNull"
    )]
    Files,
    #[sea_orm(has_many = "super::server_log::Entity")]
    ServerLog,
    #[sea_orm(has_many = "super::ticket_log::Entity")]
    TicketLog,
    #[sea_orm(has_many = "super::user_server::Entity")]
    UserServer,
}

impl Related<super::ban_records::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BanRecords.def()
    }
}

impl Related<super::files::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Files.def()
    }
}

impl Related<super::server_log::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServerLog.def()
    }
}

impl Related<super::ticket_log::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TicketLog.def()
    }
}

impl Related<super::user_server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserServer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
