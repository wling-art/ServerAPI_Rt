use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TicketStatus {
    #[sea_orm(num_value = 0)]
    Canceled,
    #[sea_orm(num_value = 1)]
    Pending,
    #[sea_orm(num_value = 2)]
    UnderReview,
    #[sea_orm(num_value = 3)]
    Resolved,
    #[sea_orm(num_value = 4)]
    Accepted,
    #[sea_orm(num_value = 5)]
    Invalid,
}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TicketType {
    #[sea_orm(num_value = 1)]
    Bug,
    #[sea_orm(num_value = 2)]
    Consult,
    #[sea_orm(num_value = 3)]
    FeatureRequest,
    #[sea_orm(num_value = 4)]
    Report,
    #[sea_orm(num_value = 5)]
    ServerIssue,
    #[sea_orm(num_value = 6)]
    ServerConfig,
    #[sea_orm(num_value = 7)]
    Other,
}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TicketPriority {
    #[sea_orm(num_value = 1)]
    Low,
    #[sea_orm(num_value = 2)]
    Medium,
    #[sea_orm(num_value = 3)]
    High,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "ticket")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub description: Option<String>,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub ticket_type: TicketType,
    pub creator_id: i32,
    pub assignee_id: Option<i32>,
    pub server_id: Option<i32>,
    pub reported_user_id: Option<i32>,
    pub reported_content_id: Option<i32>,
    #[sea_orm(column_type = "Text")]
    pub report_reason: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub admin_remark: Option<String>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: ChronoDateTimeUtc,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatorId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Creator,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::AssigneeId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Assignee,
    #[sea_orm(
        belongs_to = "super::server::Entity",
        from = "Column::ServerId",
        to = "super::server::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Server,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ReportedUserId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ReportedUser,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl Related<super::server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Server.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
