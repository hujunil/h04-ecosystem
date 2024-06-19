use anyhow::Result;
use sqlx::PgPool;

use super::{entity::UrlRecord, error::ShortenerError};

#[derive(Debug, Clone)]
pub struct AppState {
    db: PgPool,
    server_url: &'static str,
}

impl AppState {
    pub async fn try_new(db_url: &str, server_url: &'static str) -> Result<Self, ShortenerError> {
        let db = PgPool::connect(db_url).await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS urls (
                id CHAR(6) PRIMARY KEY,
                url TEXT NOT NULL UNIQUE,
                search_count BIGINT NOT NULL DEFAULT 0,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create table: {:?}", e);
            ShortenerError::InternalServerError
        })?;

        Ok(Self { db, server_url })
    }

    pub async fn shorten(&self, url: &str) -> Result<String, ShortenerError> {
        let mut loop_count = 5;
        loop {
            if loop_count == 0 {
                tracing::error!(
                    "Failed to shorten URL: too many conflicts occurred, url: {:?}",
                    url
                );
                return Err(ShortenerError::IdConflict);
            }
            loop_count -= 1;

            let id = self.create_id();
            let ret = sqlx::query_as::<_, UrlRecord>("INSERT INTO urls (id, url) VALUES ($1, $2) ON CONFLICT(url) DO UPDATE SET url=EXCLUDED.url RETURNING id, url")
            .bind(&id)
            .bind(url)
            .fetch_one(&self.db)
            .await;

            match ret {
                Ok(ret) => {
                    tracing::info!("Shortened URL: {:?}", ret);
                    return Ok(ret.id);
                }
                Err(e) => match e {
                    sqlx::Error::Database(e)
                        if e.code().as_deref() == Some("23505")
                            && Some("urls_pkey") == e.constraint() =>
                    {
                        continue;
                    }
                    _ => {
                        tracing::error!("Failed to shorten URL: {:?}", e);
                        return Err(ShortenerError::SqlxError(e));
                    }
                },
            }
        }
    }

    pub async fn find_url(&self, id: &str) -> Result<String, ShortenerError> {
        let record = sqlx::query_as::<_, UrlRecord>("SELECT url FROM urls WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await;

        let url = match record {
            Ok(Some(record)) => {
                tracing::info!("Found URL: {:?}", record.url);
                record.url
            }
            Ok(None) => {
                tracing::error!("URL not found: {:?}", id);
                return Err(ShortenerError::NotFound);
            }
            Err(e) => {
                tracing::error!("Failed to find URL: {:?}", e);
                return Err(ShortenerError::SqlxError(e));
            }
        };

        sqlx::query("UPDATE urls SET search_count = search_count + 1, updated_at = CURRENT_TIMESTAMP  WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(url)
    }

    pub fn server_url(&self) -> &'static str {
        self.server_url
    }

    fn create_id(&self) -> String {
        nanoid::nanoid!(6)
    }
}
