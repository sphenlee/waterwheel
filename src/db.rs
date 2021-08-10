use crate::config;
use sqlx::{Executor, PgPool};
use tracing::{debug, info, trace, warn};

const SCHEMA: &str = include_str!("schema.sql");

static DB_POOL: once_cell::sync::OnceCell<PgPool> = once_cell::sync::OnceCell::new();

pub async fn create_pool() -> anyhow::Result<()> {
    info!("connecting to database...");

    let url = &config::get().db_url;
    let pool = PgPool::connect(url).await?;

    let mut conn = pool.acquire().await?;
    debug!("creating schema if needed");
    let done = conn.execute(SCHEMA).await?;
    trace!("schema created: {} rows modified", done.rows_affected());

    info!("connected to database");

    if DB_POOL.set(pool).is_err() {
        warn!("database was already connected!");
    }

    Ok(())
}

pub fn get_pool() -> PgPool {
    // pools internally use Arc so clone here is cheap
    DB_POOL.get().expect("pool not created yet!").clone()
}
