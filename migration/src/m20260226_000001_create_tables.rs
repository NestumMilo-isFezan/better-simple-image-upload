use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create signed_urls table
        manager
            .create_table(
                Table::create()
                    .table(SignedUrls::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SignedUrls::Id).uuid().primary_key())
                    .col(ColumnDef::new(SignedUrls::Token).uuid().not_null().unique_key())
                    .col(ColumnDef::new(SignedUrls::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        // Create images table
        manager
            .create_table(
                Table::create()
                    .table(Images::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Images::Id).uuid().primary_key())
                    .col(ColumnDef::new(Images::TokenId).uuid().not_null())
                    .col(ColumnDef::new(Images::FilePath).string().not_null())
                    .col(ColumnDef::new(Images::Metadata).json())
                    .col(ColumnDef::new(Images::Formats).json())
                    .col(ColumnDef::new(Images::CreatedAt).timestamp_with_time_zone().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-image-token")
                            .from(Images::Table, Images::TokenId)
                            .to(SignedUrls::Table, SignedUrls::Token)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Images::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(SignedUrls::Table).to_owned())
            .await?;

        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum SignedUrls {
    Table,
    Id,
    Token,
    CreatedAt,
}

#[derive(Iden)]
enum Images {
    Table,
    Id,
    TokenId,
    FilePath,
    Metadata,
    Formats,
    CreatedAt,
}
