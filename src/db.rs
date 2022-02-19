use crate::config;
use sqlx::{Executor, PgPool};
use sqlx::postgres::PgPoolOptions;
use tracing::{debug, info, trace, warn};
use crate::config::Config;

const SCHEMA: &str = include_str!("schema.sql");

pub async fn create_pool(config: &Config) -> anyhow::Result<PgPool> {
    info!("connecting to database...");

    #[cfg(test)]
    let pool = PgPoolOptions::new()
        .after_connect(|conn| Box::pin(async move {
            conn.execute("SET search_path=pg_temp").await?;
            Ok(())
        })).connect(&config.db_url).await?;

    #[cfg(not(test))]
    let pool = PgPool::connect(&config.db_url).await?;

    let mut conn = pool.acquire().await?;
    debug!("creating schema if needed");
    let done = conn.execute(SCHEMA).await?;
    trace!("schema created: {} rows modified", done.rows_affected());

    info!("connected to database");

    Ok(pool)
}
