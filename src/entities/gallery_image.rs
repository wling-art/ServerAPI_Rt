use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(table_name = "gallery_image")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub gallery_id: i32,
    pub title: String,
    pub description: String,
    pub image_hash_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::gallery::Entity",
        from = "Column::GalleryId",
        to = "super::gallery::Column::Id"
    )]
    Gallery,
    #[sea_orm(
        belongs_to = "super::file::Entity",
        from = "Column::ImageHashId",
        to = "super::file::Column::HashValue"
    )]
    File,
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Gallery.def()
    }
}

impl Related<super::file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::File.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
