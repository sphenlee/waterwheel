use crate::{amqp, config::Config, db, metrics, server::api::jwt::JwtKeys};
use anyhow::Result;
use cadence::StatsdClient;
use lapin::Channel;
use sqlx::PgPool;
use std::{ops::Deref, path::Path, sync::Arc};
use tracing::{debug, warn};
use axum::routing::{get, post, put, delete, get_service};
use tower_http::{services::{ServeDir, ServeFile}, trace::TraceLayer};
use axum::http::StatusCode;

pub mod auth;
mod config_cache;
mod error;
mod heartbeat;
mod job;
pub mod jwt;
mod project;
mod schedulers;
mod stash;
mod status;
mod task;
mod task_logs;
pub mod types;
mod updates;
mod workers;

pub struct AppInner {
    db_pool: PgPool,
    //amqp_conn: Connection,
    amqp_channel: Channel,
    //pub post_office: PostOffice,
    statsd: Arc<StatsdClient>,
    redis_client: redis::Client,
    pub config: Config,
    pub jwt_keys: JwtKeys,
}

#[derive(Clone)]
pub struct App(Arc<AppInner>);

impl Deref for App {
    type Target = AppInner;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Into<App> for AppInner {
    fn into(self) -> App {
        App(Arc::new(self))
    }
}

impl App {
    fn get_pool(&self) -> PgPool {
        self.0.db_pool.clone()
    }

    fn get_channel(&self) -> &Channel {
        &self.0.amqp_channel
    }

    fn get_statsd(&self) -> &StatsdClient {
        &self.0.statsd
    }
}

pub type AppResult<T> = Result<T, error::AppError>;

const UI_RELATIVE_PATH: &str = "ui/dist/";

pub async fn make_app(config: Config) -> Result<axum::Router<()>> {
    let amqp_conn = amqp::amqp_connect(&config).await?;
    let db_pool = db::create_pool(&config).await?;
    let statsd = metrics::new_client(&config)?;
    let jwt_keys = jwt::load_keys(&config)?;

    let amqp_channel = amqp_conn.create_channel().await?;
    let redis_client = redis::Client::open(config.redis_url.as_ref())?;

    let app = AppInner {
        config,
        db_pool,
        //amqp_conn,
        amqp_channel,
        statsd,
        jwt_keys,
        redis_client,
    };

    updates::setup(&app.amqp_channel).await?;
    config_cache::setup(&app.amqp_channel).await?;

    let router = axum::Router::new()
        // enable request tracing for all routes
        .layer(TraceLayer::new_for_http())

        // healthcheck
        .route("/healthcheck", get(|| async { "OK" }))

        // status
        .route("/api/status", get(status::status))

        // worker heartbeats
        .route("/int-api/heartbeat", post(heartbeat::post))

        // project
        .route(
            "/api/projects",
            get(project::get_by_name)
                .post(project::create)
                .put(project::create),
        )
        .route(
            "/api/projects/{id}",
            get(project::get_by_id).delete(project::delete),
        )
        .route("/api/projects/{id}/jobs", get(project::list_jobs))
        .route("/int-api/projects/{id}/config", get(project::get_config))

        // project stash
        .route("/api/projects/{id}/stash", get(stash::project::list))
        .route(
            "/api/projects/{id}/stash/{key}",
            put(stash::project::create).delete(stash::project::delete),
        )
        .route("/int-api/projects/{id}/stash/{key}", get(stash::project::get))

        // job
        .route(
            "/api/jobs",
            get(job::get_by_name)
                .post(job::create)
                .put(job::create),
        )
        .route(
            "/api/jobs/{id}",
            get(job::get_by_id).delete(job::delete),
        )
        .route("/api/jobs/{id}/tasks", get(job::list_tasks))
        .route(
            "/api/jobs/{id}/paused",
            get(job::get_paused).put(job::set_paused),
        )
        .route("/api/jobs/{id}/graph", get(job::get_graph))
        .route("/api/jobs/{id}/duration", get(job::get_duration))

        // job tokens
        .route("/api/jobs/{id}/tokens", get(job::get_tokens))
        .route("/api/jobs/{id}/tokens-overview", get(job::get_tokens_overview))
        .route(
            "/api/jobs/{id}/tokens/{trigger_datetime}",
            get(job::get_tokens_trigger_datetime)
                .delete(job::clear_tokens_trigger_datetime),
        )

        // job runs
        .route(
            "/api/jobs/:id/runs/:trigger_datetime",
            get(job::list_job_all_task_runs),
        )

        // job triggers
        .route("/api/jobs/:id/triggers", get(job::get_triggers_by_job))

        // job stash
        .route(
            "/int-api/jobs/:id/stash/:trigger_datetime/",
            get(stash::job::list),
        )
        .route(
            "/int-api/jobs/:id/stash/:trigger_datetime/:key",
            put(stash::job::create)
                .get(stash::job::get)
                .delete(stash::job::delete),
        )

        // tasks
        .route("/api/tasks/:id", get(task::get_task_def))
        .route(
            "/api/tasks/:id/tokens",
            post(task::activate_multiple_tokens),
        )
        .route(
            "/api/tasks/:id/tokens/:trigger_datetime",
            put(task::activate_token),
        )
        .route("/int-api/tasks/:id", get(task::internal_get_task_def))

        // task runs
        .route(
            "/api/tasks/:id/runs/:trigger_datetime",
            get(job::list_task_runs),
        )

        // task logs (websocket handler)
        .route("/api/task_runs/:id/logs", get(task_logs::logs))

        // trigger times
        .route("/api/triggers/:id", get(job::get_trigger))

        // workers
        .route("/api/workers", get(workers::list))
        .route("/api/workers/:id", get(workers::tasks))

        // schedulers
        .route("/api/schedulers", get(schedulers::list))

        // stash
        .route("/api/stash", get(stash::global::list))
        .route(
            "/api/stash/:key",
            put(stash::global::create).delete(stash::global::delete),
        )
        .route("/int-api/stash/:key", get(stash::global::get))

        // static files
        .nest_service(
            "/static",
            ServeDir::new(UI_RELATIVE_PATH),
        )
        .fallback_service(ServeFile::new(Path::new(UI_RELATIVE_PATH).join("index.html")))

        .with_state(app.into());

    Ok(router)
}

pub async fn serve(config: Config) -> Result<()> {
    if config.no_authz {
        warn!("authorization is disabled, this is not recommended in production");
    }
    let server_bind = config.server_bind.clone();

    let app = make_app(config).await?;
    
    debug!("server binding to {}", server_bind);

    let listener = tokio::net::TcpListener::bind(server_bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
