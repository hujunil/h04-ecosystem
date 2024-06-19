use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct UrlRecord {
    #[sqlx(default)]
    pub id: String,
    #[sqlx(default)]
    pub url: String,
}
