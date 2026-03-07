use crate::{
    config::Config,
    server::api::{App, AppResult, error::AppError, job::get_job_project_id},
};
use axum::http::{HeaderMap, StatusCode};
use axum_extra::headers::{Authorization, HeaderMapExt, authorization::Bearer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, warn};
use uuid::Uuid;

#[derive(Serialize, Debug, Default)]
struct Object {
    project_id: Option<Uuid>,
    job_id: Option<Uuid>,
    kind: String,
}

#[derive(Serialize, Debug)]
pub struct Principal {
    bearer: Option<String>, // bearer token if present
}

#[derive(Serialize, Debug, Copy, Clone)]
pub enum Action {
    Get,
    List,
    Update,
    Delete,
}

#[derive(Serialize, Debug)]
struct Http {
    headers: HashMap<String, String>,
}

#[derive(Serialize)]
struct RequestCtx<'a> {
    object: &'a Object,
    principal: &'a Principal,
    action: Action,
    http: Http,
}

#[derive(Serialize)]
struct OPARequest<'a> {
    input: RequestCtx<'a>,
}

#[derive(Deserialize)]
struct OPAResponse {
    result: Option<bool>,
}

fn derive_principal(headers: &axum::http::HeaderMap) -> anyhow::Result<Principal> {
    if let Some(bearer) = headers.typed_get::<Authorization<Bearer>>() {
        Ok(Principal {
            bearer: Some(bearer.0.token().to_owned()),
        })
    } else {
        Ok(Principal { bearer: None })
    }
}

fn derive_http(headers: &HeaderMap) -> anyhow::Result<Http> {
    let mut header_map = HashMap::new();

    for (k, v) in headers {
        if let Ok(val) = v.to_str() {
            // TODO avoid this copying
            header_map.insert(k.to_string(), val.to_owned());
        }
    }

    Ok(Http {
        headers: header_map,
    })
}

async fn authorize(
    config: &Config,
    principal: Principal,
    action: Action,
    object: Object,
    http: Http,
) -> AppResult<bool> {
    let opa = if let Some(opa) = config.opa_sidecar_addr.as_ref() {
        opa
    } else {
        error!(
            "OPA sidecar address is unset (to disable authz you must set `WATERWHEEL_NO_AUTHZ=true`)"
        );
        return Ok(false);
    };

    let url = opa.join("/v1/data/waterwheel/authorize")?;

    let reply = reqwest::Client::new()
        .post(url)
        .json(&OPARequest {
            input: RequestCtx {
                principal: &principal,
                action,
                object: &object,
                http,
            },
        })
        .send()
        .await?;

    let result: OPAResponse = reply.json().await?;

    // purposely don't log the HTTP object as it contains raw headers which could contain tokens or cookies
    if result.result.unwrap_or(false) {
        debug!(?principal, ?action, ?object, "authorized");
    } else {
        warn!(?principal, ?action, ?object, "unauthorized");
    }

    Ok(result.result.unwrap_or(false))
}

pub struct Check<'a> {
    app: &'a App,
    action: Action,
    object: Object,
}

impl Check<'_> {
    pub fn project(mut self, project_id: impl Into<Option<Uuid>>) -> Self {
        self.object.project_id = project_id.into();
        self.object.kind = "project".to_owned();
        self
    }

    pub fn job(
        mut self,
        job_id: impl Into<Option<Uuid>>,
        proj_id: impl Into<Option<Uuid>>,
    ) -> Self {
        self.object.job_id = job_id.into();
        self.object.project_id = proj_id.into();
        self.object.kind = "job".to_owned();
        self
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.object.kind = kind.into();
        self
    }

    pub async fn check(self, headers: &HeaderMap) -> AppResult<()> {
        if self.app.0.config.no_authz {
            return Ok(());
        }

        let principal = derive_principal(headers)?;
        let mut object = self.object;

        if let Some(job_id) = object.job_id
            && object.project_id.is_none()
        {
            let pool = self.app.get_pool();
            let project_id = get_job_project_id(&pool, job_id).await?;
            object.project_id = Some(project_id);
        }

        let http = derive_http(headers)?;
        // NOTE - this potentially logs credentials so don't leave it uncommented
        //debug!("http context", { http: Value::from_debug(&http) });

        if authorize(&self.app.0.config, principal, self.action, object, http).await? {
            Ok(())
        } else {
            Err(AppError::http(StatusCode::FORBIDDEN))
        }
    }
}

pub fn get(app: &App) -> Check<'_> {
    Check {
        app,
        action: Action::Get,
        object: Default::default(),
    }
}

pub fn list(app: &App) -> Check<'_> {
    Check {
        app,
        action: Action::List,
        object: Default::default(),
    }
}

pub fn update(app: &App) -> Check<'_> {
    Check {
        app,
        action: Action::Update,
        object: Default::default(),
    }
}

pub fn delete(app: &App) -> Check<'_> {
    Check {
        app,
        action: Action::Delete,
        object: Default::default(),
    }
}
