use anyhow::Result;
use serde::Deserialize;
use uuid::Uuid;
use serde::Serialize;
use crate::config;
use crate::server::api::request_ext::RequestExt;
use highnoon::headers::Authorization;
use highnoon::headers::authorization::Bearer;
use highnoon::StatusCode;
use kv_log_macro::debug;
use crate::server::api::project::get_project_name;
use crate::server::api::State;
use crate::server::api::job::get_job_name_and_project;
use log::kv::Value;
use std::collections::HashMap;


#[derive(Serialize, Debug)]
struct Object {
    project_id: Option<Uuid>,
    project_name: Option<String>,
    job_id: Option<Uuid>,
    job_name: Option<String>,
    kind: String,
}

#[derive(Serialize, Debug)]
pub struct Principal {
    bearer: Option<String> // bearer token if present
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
    result: bool
}

fn derive_principal<S: highnoon::State>(req: &highnoon::Request<S>) -> Result<Principal> {
    let bearer = req.header::<Authorization<Bearer>>().map(|header| {
        header.0.token().to_owned()
    });

    Ok(Principal {
        bearer,
    })
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
        headers
    })
}

async fn authorize(principal: Principal, action: Action, object: Object, http: Http) -> Result<bool> {
    let opa = match config::get().opa_sidecar_addr.as_ref() {
        // TODO - allow by default if OPA address is not set, not very secure default
        None => return Ok(true),
        Some(url) => url
    };

    let url = opa.join("/v1/data/waterwheel/allow")?;

    let reply = reqwest::Client::new()
        .post(url)
        .json(&OPARequest {
            input: RequestCtx {
                principal: &principal,
                action,
                object: &object,
                http
            }
        })
        .send()
        .await?;

    let result: OPAResponse = reply.json().await?;

    // purposely don't log the HTTP object as it contains raw headers which could contain tokens or cookies
    if result.result {
        debug!("authorized", {
            principal: Value::from_debug(&principal),
            action: Value::from_debug(&action),
            object: Value::from_debug(&object)
        });
    } else {
        debug!("unauthorized", {
            principal: Value::from_debug(&principal),
            action: Value::from_debug(&action),
            object: Value::from_debug(&object)
        });
    }


    Ok(result.result)
}

// TODO - playing with the ergonomics here
pub struct Check {
    action: Action,
    object: Option<Object>
}

impl Check {
    pub fn project(mut self, project_id: impl Into<Option<Uuid>>) -> Self {
        self.object = Some(Object {
            project_id: project_id.into(),
            project_name: None,
            job_id: None,
            job_name: None,
            kind: "project".to_owned(),
        });
        self
    }

    pub fn job(mut self, job_id: impl Into<Option<Uuid>>) -> Self {
        self.object = Some(Object {
            project_id: None,//Some(proj_id),
            project_name: None,
            job_id: job_id.into(),
            job_name: None,
            kind: "job".to_owned(),
        });
        self
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.object = Some(Object {
            project_id: None,
            project_name: None,
            job_id: None,
            job_name: None,
            kind: kind.into(),
        });
        self
    }

    pub async fn check(self, req: &highnoon::Request<State>) -> highnoon::Result<()> {
        let principal = derive_principal(req)?;
        let mut object = self.object.expect("authorization object uninitialised");

        // TODO - DB query on every auth decision; maybe cache project/job id -> name mappings?
        let pool = req.get_pool();

        if let Some(proj_id) = object.project_id {
            let name = get_project_name(&pool, proj_id).await?;
            object.project_name = Some(name);
        }

        if let Some(job_id) = object.job_id {
            let jp = get_job_name_and_project(&pool, job_id).await?;
            object.project_id = Some(jp.project_id);
            object.project_name = Some(jp.project_name);
            object.job_name = Some(jp.job_name);
        }

        let http = derive_http(&req)?;
        //debug!("http context", { http: Value::from_debug(&http) });

        if authorize(principal, self.action, object, http).await? {
            Ok(())
        } else {
            Err(highnoon::Error::http(StatusCode::FORBIDDEN))
        }
    }
}

pub fn get() -> Check {
    Check { action: Action::Get, object: None }
}

pub fn list() -> Check {
    Check { action: Action::List, object: None }
}

pub fn update() -> Check {
    Check { action: Action::Update, object: None }
}

pub fn delete() -> Check {
    Check { action: Action::Delete, object: None }
}
