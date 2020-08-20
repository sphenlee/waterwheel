use anyhow::Result;
use sqlx::postgres::PgDatabaseError;
use sqlx::PgPool;

mod job;
mod project;
pub mod types;
pub mod util;

const PG_INTEGRITY_ERROR: &str = "23";

#[derive(Clone)]
pub struct State {
    pool: PgPool,
}

pub async fn serve() -> Result<()> {
    let state = State {
        pool: crate::db::get_pool(),
    };

    let mut app = tide::with_state(state);

    app.at("/")
        .get(|_req| async { Ok("Hello from Waterwheel!") });

    // project
    app.at("/api/project")
        .get(project::get_by_name)
        .put(project::create);
    app.at("/api/project/:id")
        .get(project::get_by_id)
        .delete(project::delete);

    // job
    app.at("/api/job")
        .get(job::get_by_name)
        .post(job::create)
        .put(job::create);
    app.at("/api/job/:id")
        .get(job::get_by_id)
        .delete(job::delete);


    let host =
        std::env::var("WATERWHEEL_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());

    app.listen(host).await?;

    Ok(())
}

pub fn pg_error<T>(res: sqlx::Result<T>) -> Result<std::result::Result<T, Box<PgDatabaseError>>> {
    match res {
        Ok(t) => Ok(Ok(t)),
        Err(err) => match err {
            sqlx::Error::Database(db_err) => Ok(Err(db_err.downcast::<PgDatabaseError>())),
            err => Err(err.into()),
        },
    }
}
