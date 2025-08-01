//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.14

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ticket_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub old_status: i16,
    pub new_status: i16,
    pub changed_at: DateTime,
    pub changed_by_id: i32,
    pub ticket_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ticket::Entity",
        from = "Column::TicketId",
        to = "super::ticket::Column::Id",
        on_update = "Restrict",
        on_delete = "Cascade"
    )]
    Ticket,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::ChangedById",
        to = "super::users::Column::Id",
        on_update = "Restrict",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::ticket::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ticket.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
