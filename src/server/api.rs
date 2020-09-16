use anyhow::Result;
use hightide::wrap;
use sqlx::postgres::PgDatabaseError;
use sqlx::PgPool;

mod job;
mod project;
mod task;
pub mod types;
pub mod util;
mod workers;

const PG_INTEGRITY_ERROR: &str = "23";

#[derive(Clone)]
pub struct State {
    pool: PgPool,
}

#[allow(unused)]
macro_rules! get_file {
    ($file:expr => $mime:expr) => {
        |_req| async {
            let mut body = tide::Body::from(include_str!($file));
            body.set_mime($mime);
            Ok(body)
        }
    };
}

pub async fn serve() -> Result<()> {
    let state = State {
        pool: crate::db::get_pool(),
    };

    let mut app = tide::with_state(state);
    app.with(tide::log::LogMiddleware::new());

    // project
    app.at("/api/projects")
        .get(wrap(project::get_by_name))
        .post(wrap(project::create))
        .put(wrap(project::update));
    app.at("/api/projects/:id")
        .get(wrap(project::get_by_id))
        .delete(wrap(project::delete));
    app.at("/api/projects/:id/jobs")
        .get(wrap(project::list_jobs));

    // job
    app.at("/api/jobs")
        .get(wrap(job::get_by_name))
        .post(wrap(job::create))
        .put(wrap(job::create));
    app.at("/api/jobs/:id")
        .get(wrap(job::get_by_id))
        .delete(wrap(job::delete));

    // job tokens
    app.at("/api/jobs/:id/tokens").get(wrap(job::get_tokens));
    app.at("/api/jobs/:id/tokens/:trigger_datetime")
        .get(wrap(job::get_tokens_trigger_datetime))
        .delete(wrap(job::clear_tokens_trigger_datetime));

    // job triggers
    app.at("/api/jobs/:id/triggers")
        .get(wrap(job::get_triggers_by_job));
    app.at("/api/jobs/:id/graph").get(wrap(job::get_graph));
    app.at("/api/jobs/:job_id/triggers/:id")
        .get(wrap(job::get_trigger));

    // task tokens
    app.at("/api/tasks/:id/tokens/:trigger_datetime")
        .put(wrap(task::create_token));

    // trigger times
    app.at("/api/triggers/:id")
        .get(wrap(job::get_trigger_times));

    // workers
    app.at("/api/workers").get(wrap(workers::list));
    app.at("/api/workers/:id").get(wrap(workers::tasks));

    // web UI

    #[cfg(debug_assertions)]
    {
        app.at("/static").serve_dir("ui/dist/")?;
        app.at("/").get(|_req| async {
            let body = tide::Body::from_file("ui/dist/index.html").await?;
            Ok(body)
        });
    }

    #[cfg(not(debug_assertions))]
    {
        app.at("/static/main.js")
            .get(get_file!("../../ui/dist/main.js" => "text/javascript"));
        app.at("/")
            .get(get_file!("../../ui/dist/index.html" => "text/html;charset=utf-8"));
    }

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
