use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "gallery")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::server::Entity")]
    Servers,
    #[sea_orm(has_many = "super::gallery_image::Entity")]
    GalleryImages,
}

impl Related<super::server::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Servers.def()
    }
}

impl Related<super::gallery_image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GalleryImages.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
