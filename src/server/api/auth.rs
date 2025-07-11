use crate::{
    config::Config,
    server::api::{job::get_job_project_id, request_ext::RequestExt, State},
};
use anyhow::Result;
use highnoon::{
    headers::{authorization::Bearer, Authorization},
    StatusCode,
};
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
    method: String,
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

fn derive_principal<S: highnoon::State>(req: &highnoon::Request<S>) -> Result<Principal> {
    let bearer = req
        .header::<Authorization<Bearer>>()
        .map(|header| header.0.token().to_owned());

    Ok(Principal { bearer })
}

fn derive_http<S: highnoon::State>(req: &highnoon::Request<S>) -> Result<Http> {
    let mut headers = HashMap::new();

    for (k, v) in req.headers() {
        if let Ok(val) = v.to_str() {
            // TODO avoid this copying
            headers.insert(k.to_string(), val.to_owned());
        }
    }

    Ok(Http {
        method: req.method().to_string(),
        headers,
    })
}

async fn authorize(
    config: &Config,
    principal: Principal,
    action: Action,
    object: Object,
    http: Http,
) -> highnoon::Result<bool> {
    let opa = if let Some(opa) = config.opa_sidecar_addr.as_ref() {
        opa
    } else {
        error!("OPA sidecar address is unset (to disable authz you must set `WATERWHEEL_NO_AUTHZ=true`)");
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

pub struct Check {
    action: Action,
    object: Object,
}

impl Check {
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

    pub async fn check(self, req: &highnoon::Request<State>) -> highnoon::Result<()> {
        let config = &req.state().config;
        if config.no_authz {
            return Ok(());
        }

        let principal = derive_principal(req)?;
        let mut object = self.object;

        if let Some(job_id) = object.job_id
            && object.project_id.is_none() {
                let pool = req.get_pool();
                let project_id = get_job_project_id(&pool, job_id).await?;
                object.project_id = Some(project_id);
            }

        let http = derive_http(req)?;
        // NOTE - this potentially logs credentials so don't leave it uncommented
        //debug!("http context", { http: Value::from_debug(&http) });

        if authorize(config, principal, self.action, object, http).await? {
            Ok(())
        } else {
            Err(highnoon::Error::http(StatusCode::FORBIDDEN))
        }
    }
}

pub fn get() -> Check {
    Check {
        action: Action::Get,
        object: Default::default(),
    }
}

pub fn list() -> Check {
    Check {
        action: Action::List,
        object: Default::default(),
    }
}

pub fn update() -> Check {
    Check {
        action: Action::Update,
        object: Default::default(),
    }
}

pub fn delete() -> Check {
    Check {
        action: Action::Delete,
        object: Default::default(),
    }
}
