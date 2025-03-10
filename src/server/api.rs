use crate::{amqp, config::Config, db, metrics, server::api::jwt::JwtKeys};
use anyhow::{Context, Result};
use cadence::StatsdClient;
use lapin::Channel;
use sqlx::PgPool;
use std::{path::Path, sync::Arc};
use tracing::{debug, warn};

pub mod auth;
mod config_cache;
mod heartbeat;
mod job;
pub mod jwt;
mod project;
mod request_ext;
mod schedulers;
mod stash;
mod status;
mod task;
mod task_logs;
pub mod types;
mod updates;
mod workers;

pub struct State {
    db_pool: PgPool,
    //amqp_conn: Connection,
    amqp_channel: Channel,
    //pub post_office: PostOffice,
    statsd: Arc<StatsdClient>,
    redis_client: redis::Client,
    pub config: Config,
    pub jwt_keys: JwtKeys,
}

impl highnoon::State for State {
    type Context = ();
    fn new_context(&self) -> Self::Context {
        Self::Context::default()
    }
}

const UI_RELATIVE_PATH: &str = "ui/dist/";

pub async fn make_app(config: Config) -> Result<highnoon::App<State>> {
    std::fs::metadata(UI_RELATIVE_PATH).context("ui resources not found")?;

    let amqp_conn = amqp::amqp_connect(&config).await?;
    let db_pool = db::create_pool(&config).await?;
    let statsd = metrics::new_client(&config)?;
    let jwt_keys = jwt::load_keys(&config)?;

    let amqp_channel = amqp_conn.create_channel().await?;
    let redis_client = redis::Client::open(config.redis_url.as_ref())?;

    let state = State {
        config,
        db_pool,
        //amqp_conn,
        amqp_channel,
        statsd,
        jwt_keys,
        redis_client,
    };

    updates::setup(&state.amqp_channel).await?;
    config_cache::setup(&state.amqp_channel).await?;

    let mut app = highnoon::App::new(state);
    app.with(highnoon::filter::Log);

    // basic healthcheck to see if waterwheel is up
    app.at("/healthcheck").get(|_req| async { Ok("OK") });

    app.at("/api/status").get(status::status);

    // worker heartbeats
    app.at("/int-api/heartbeat").post(heartbeat::post);

    // project
    app.at("/api/projects")
        .get(project::get_by_name)
        .post(project::create)
        .put(project::create);
    app.at("/api/projects/:id")
        .get(project::get_by_id)
        .delete(project::delete);
    app.at("/api/projects/:id/jobs").get(project::list_jobs);

    app.at("/int-api/projects/:id/config")
        .get(project::get_config);

    // project stash
    app.at("/api/projects/:id/stash").get(stash::project::list);
    app.at("/api/projects/:id/stash/:key")
        .put(stash::project::create)
        .delete(stash::project::delete);

    app.at("/int-api/projects/:id/stash/:key")
        .get(stash::project::get);

    // job
    app.at("/api/jobs")
        .get(job::get_by_name)
        .post(job::create)
        .put(job::create);
    app.at("/api/jobs/:id")
        .get(job::get_by_id)
        .delete(job::delete);
    app.at("/api/jobs/:id/tasks").get(job::list_tasks);
    app.at("/api/jobs/:id/paused")
        .get(job::get_paused)
        .put(job::set_paused);
    app.at("/api/jobs/:id/graph").get(job::get_graph);
    app.at("/api/jobs/:id/duration").get(job::get_duration);

    // job tokens
    app.at("/api/jobs/:id/tokens").get(job::get_tokens);
    app.at("/api/jobs/:id/tokens-overview")
        .get(job::get_tokens_overview);
    app.at("/api/jobs/:id/tokens/:trigger_datetime")
        .get(job::get_tokens_trigger_datetime)
        .delete(job::clear_tokens_trigger_datetime);

    // job runs
    app.at("/api/jobs/:id/runs/:trigger_datetime")
        .get(job::list_job_all_task_runs);

    // job triggers
    app.at("/api/jobs/:id/triggers")
        .get(job::get_triggers_by_job);

    // job stash
    app.at("/int-api/jobs/:id/stash/:trigger_datetime/")
        .get(stash::job::list);
    app.at("/int-api/jobs/:id/stash/:trigger_datetime/:key")
        .put(stash::job::create)
        .get(stash::job::get)
        .delete(stash::job::delete);

    // tasks
    app.at("/api/tasks/:id").get(task::get_task_def);
    app.at("/api/tasks/:id/tokens")
        .post(task::activate_multiple_tokens);
    app.at("/api/tasks/:id/tokens/:trigger_datetime")
        .put(task::activate_token);
    app.at("/int-api/tasks/:id")
        .get(task::internal_get_task_def);

    // task runs
    app.at("/api/tasks/:id/runs/:trigger_datetime")
        .get(job::list_task_runs);

    // task logs - TODO unimplemented
    app.at("/api/task_runs/:id/logs").ws(task_logs::logs);

    // trigger times
    app.at("/api/triggers/:id").get(job::get_trigger);

    // workers
    app.at("/api/workers").get(workers::list);
    app.at("/api/workers/:id").get(workers::tasks);

    // schedulers
    app.at("/api/schedulers").get(schedulers::list);

    // stash
    app.at("/api/stash").get(stash::global::list);
    app.at("/api/stash/:key")
        .put(stash::global::create)
        .delete(stash::global::delete);

    app.at("/int-api/stash/:key").get(stash::global::get);

    // web UI

    let index = |_req: highnoon::Request<State>| async {
        let body = highnoon::Response::ok().path(Path::new(UI_RELATIVE_PATH).join("index.html")).await?;
        Ok(body)
    };
    app.at("/static/*").static_files(UI_RELATIVE_PATH);
    app.at("/**").get(index);
    app.at("/").get(index);

    Ok(app)
}

pub async fn serve(config: Config) -> Result<()> {
    if config.no_authz {
        warn!("authorization is disabled, this is not recommended in production");
    }

    let app = make_app(config).await?;

    let server_bind = &app.state().config.server_bind.clone();
    debug!("server binding to {}", server_bind);
    app.listen(&server_bind).await?;

    Ok(())
}
