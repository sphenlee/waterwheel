use super::status::SERVER_STATUS;
use anyhow::Result;
use sqlx::PgPool;
use highnoon::{Request, Responder};

mod job;
mod project;
pub mod request_ext;
mod task;
pub mod types;
mod workers;

#[derive(Clone)]
pub struct State {
    pool: PgPool,
}

#[allow(unused)]
macro_rules! get_file {
    ($file:expr => $mime:expr) => {
        |_req| async {
            use highnoon::{Response, headers::ContentType, Mime};

            let body = include_str!($file);
            Response::ok().body(body)
                .header(ContentType::from($mime.parse::<Mime>().unwrap()))
        }
    };
}

pub async fn serve() -> Result<()> {
    let state = State {
        pool: crate::db::get_pool(),
    };

    let mut app = highnoon::App::new(state);
    app.with(highnoon::filter::Log);

    // basic healthcheck to see if waterwheel is up
    app.at("/healthcheck").get(|_req| async { Ok("OK") });

    app.at("/api/status").get(status);

    // project
    app.at("/api/projects")
        .get(project::get_by_name)
        .post(project::create)
        .put(project::update);
    app.at("/api/projects/:id")
        .get(project::get_by_id)
        .delete(project::delete);
    app.at("/api/projects/:id/jobs")
        .get(project::list_jobs);

    // job
    app.at("/api/jobs")
        .get(job::get_by_name)
        .post(job::create)
        .put(job::create);
    app.at("/api/jobs/:id")
        .get(job::get_by_id)
        .delete(job::delete);
    app.at("/api/jobs/:id/paused")
        .get(job::get_paused)
        .put(job::set_paused);

    // job tokens
    app.at("/api/jobs/:id/tokens").get(job::get_tokens);
    app.at("/api/jobs/:id/tokens-overview")
        .get(job::get_tokens_overview);
    app.at("/api/jobs/:id/tokens/:trigger_datetime")
        .get(job::get_tokens_trigger_datetime)
        .delete(job::clear_tokens_trigger_datetime);

    // job triggers
    app.at("/api/jobs/:id/triggers")
        .get(job::get_triggers_by_job);
    app.at("/api/jobs/:id/graph").get(job::get_graph);
    app.at("/api/jobs/:job_id/triggers/:id")
        .get(job::get_trigger);

    // task tokens
    app.at("/api/tasks/:id/tokens/:trigger_datetime")
        .put(task::create_token);

    // trigger times
    app.at("/api/triggers/:id")
        .get(job::get_trigger_times);

    // workers
    app.at("/api/workers").get(workers::list);
    app.at("/api/workers/:id").get(workers::tasks);

    // web UI

    #[cfg(debug_assertions)]
    {
        app.at("/static/*").static_files("ui/dist/");
        app.at("/").get(|_req| async {
            let body = highnoon::Response::ok()
                .path("ui/dist/index.html").await?;
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

async fn status(_req: Request<State>) -> highnoon::Result<impl Responder> {
    let status = SERVER_STATUS.lock().await;

    let json = serde_json::to_string(&*status)?;

    Ok(json)
}
