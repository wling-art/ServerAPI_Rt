use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "auth_mode_enum")]
pub enum AuthModeEnum {
    #[sea_orm(string_value = "OFFLINE")]
    Offline,
    #[sea_orm(string_value = "YGGDRASIL")]
    Yggdrasil,
    #[sea_orm(string_value = "OFFICIAL")]
    Official,
}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "server_type_enum")]
pub enum ServerTypeEnum {
    #[sea_orm(string_value = "JAVA")]
    Java,
    #[sea_orm(string_value = "BEDROCK")]
    Bedrock,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "server")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub r#type: String,
    pub version: String,
    #[sea_orm(column_type = "custom(\"LONGTEXT\")")]
    pub desc: String,
    pub link: String,
    pub ip: String,
    pub is_member: bool,
    pub is_hide: bool,
    pub auth_mode: String,
    #[sea_orm(column_type = "custom(\"LONGTEXT\")")]
    pub tags: String,
    pub cover_hash_id: Option<String>,
    pub gallery_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::files::Entity",
        from = "Column::CoverHashId",
        to = "super::files::Column::HashValue",
        on_update = "Restrict",
        on_delete = "SetNull"
    )]
    Files,
    #[sea_orm(
        belongs_to = "super::gallery::Entity",
        from = "Column::GalleryId",
        to = "super::gallery::Column::Id",
        on_update = "Restrict",
        on_delete = "Cascade"
    )]
    Gallery,
    #[sea_orm(has_many = "super::server_log::Entity")]
    ServerLog,
    #[sea_orm(has_many = "super::server_stats::Entity")]
    ServerStats,
    #[sea_orm(has_many = "super::ticket::Entity")]
    Ticket,
    #[sea_orm(has_many = "super::user_server::Entity")]
    UserServer,
}

impl Related<super::files::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Files.def()
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Gallery.def()
    }
}

impl Related<super::server_log::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServerLog.def()
    }
}

impl Related<super::server_stats::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServerStats.def()
    }
}

impl Related<super::ticket::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ticket.def()
    }
}

impl Related<super::user_server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserServer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
