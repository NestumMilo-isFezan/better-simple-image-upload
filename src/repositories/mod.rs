use crate::models::{signed_url, image};
use sea_orm::*;
use uuid::Uuid;

pub struct SignedUrlRepo;

impl SignedUrlRepo {
    pub async fn create(db: &DatabaseConnection, token: Uuid) -> Result<signed_url::Model, DbErr> {
        let active_model = signed_url::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            token: ActiveValue::Set(token),
            created_at: ActiveValue::Set(chrono::Utc::now().into()),
        };
        signed_url::Entity::insert(active_model).exec_with_returning(db).await
    }

    pub async fn find_by_token(db: &DatabaseConnection, token: Uuid) -> Result<Option<signed_url::Model>, DbErr> {
        signed_url::Entity::find()
            .filter(signed_url::Column::Token.eq(token))
            .one(db)
            .await
    }
}

pub struct ImageRepo;

impl ImageRepo {
    pub async fn create(
        db: &DatabaseConnection,
        token_id: Uuid,
        file_path: String,
        metadata: serde_json::Value,
        formats: Vec<String>,
    ) -> Result<image::Model, DbErr> {
        let active_model = image::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            token_id: ActiveValue::Set(token_id),
            file_path: ActiveValue::Set(file_path),
            metadata: ActiveValue::Set(Some(metadata)),
            formats: ActiveValue::Set(Some(serde_json::to_value(formats).unwrap())),
            created_at: ActiveValue::Set(chrono::Utc::now().into()),
        };
        image::Entity::insert(active_model).exec_with_returning(db).await
    }

    pub async fn find_by_token(db: &DatabaseConnection, token: Uuid) -> Result<Option<image::Model>, DbErr> {
        image::Entity::find()
            .filter(image::Column::TokenId.eq(token))
            .one(db)
            .await
    }

    /// Safely appends new formats to the existing JSONB array using SQL.
    pub async fn add_formats(
        db: &DatabaseConnection,
        token: Uuid,
        new_formats: Vec<String>,
    ) -> Result<(), DbErr> {
        let formats_json = serde_json::to_string(&new_formats).unwrap();
        
        // Use raw SQL for an atomic JSONB append to prevent race conditions
        let query = Statement::from_string(
            db.get_database_backend(),
            format!(
                "UPDATE images SET formats = COALESCE(formats, '[]'::jsonb) || '{}'::jsonb WHERE token_id = '{}'",
                formats_json, token
            ),
        );

        db.execute(query).await?;
        Ok(())
    }
}
