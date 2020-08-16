use sqlx::{Executor, PgPool};
use log::{info, debug, trace};

const SCHEMA: &str = include_str!("schema.sql");

pub async fn create_pool() -> anyhow::Result<PgPool> {
    info!("connecting to database...");

    let url = std::env::var("WATERWHEEL_DB_URL").expect("database URL not set");
    let pool = PgPool::new(&url).await?;

    let mut conn = pool.acquire().await?;
    debug!("creating schema if needed");
    let c = conn.execute(SCHEMA).await?;
    trace!("schema created: {} rows modified", c);

    info!("connected to database");

    Ok(pool)
}
