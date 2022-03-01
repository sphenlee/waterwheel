use crate::config::Config;
use sqlx::{Executor, PgPool};
use tracing::{debug, info, trace};

const SCHEMA: &str = include_str!("schema.sql");

pub async fn create_pool(config: &Config) -> anyhow::Result<PgPool> {
    info!("connecting to database...");

    let pool = PgPool::connect(&config.db_url).await?;

    let mut conn = pool.acquire().await?;
    debug!("creating schema if needed");
    let done = conn.execute(SCHEMA).await?;
    trace!("schema created: {} rows modified", done.rows_affected());

    info!("connected to database");

    Ok(pool)
}
