use anyhow::Result;
use serde::Deserialize;
use uuid::Uuid;
use serde::Serialize;
use reqwest::Url;
use crate::config;
use crate::server::api::request_ext::RequestExt;
use highnoon::headers::Authorization;
use highnoon::headers::authorization::Bearer;
use highnoon::StatusCode;
use kv_log_macro::debug;
use crate::server::api::project::get_project_name;
use crate::server::api::State;
use crate::server::api::job::get_job_name_and_project;

fn as_debug<D: std::fmt::Debug>(obj: &D) -> log::kv::Value {
    log::kv::Value::from_debug(obj)
}

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
    identity: String, // who is making the request
    authority: String, // who verified their identity
    groups: Vec<String>, // group membership
}

#[derive(Serialize, Debug, Copy, Clone)]
pub enum Action {
    Get,
    List,
    Update,
    Delete,
}

#[derive(Serialize)]
struct RequestCtx<'a> {
    object: &'a Object,
    principal: &'a Principal,
    action: Action,
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
    let authority = match config::get_opt::<String>("WATERWHEEL_HEADER_AUTHORITY")? {
        None => "bearer".to_owned(),
        Some(header) => {
            match req.headers().get(&header) {
                None => "none".to_owned(),
                Some(value) => value.to_str()?.to_owned()
            }
        }
    };

    let identity = match config::get_opt::<String>("WATERWHEEL_HEADER_IDENTITY")? {
        None => match req.header::<Authorization<Bearer>>() {
            None => "".to_owned(),
            Some(auth) => auth.0.token().to_owned()
        },
        Some(header) => match req.headers().get(&header) {
            None => "".to_owned(),
            Some(value) => value.to_str()?.to_owned()
        }
    };

    Ok(Principal {
        identity,
        authority,
        groups: vec![],
    })
}

async fn authorize(principal: Principal, action: Action, object: Object) -> Result<bool> {
    let opa: Url = config::get("WATERWHEEL_OPA_SIDECAR_ADDR")?;
    let url = opa.join("/v1/data/waterwheel/allow")?;

    let reply = reqwest::Client::new()
        .post(url)
        .json(&OPARequest {
            input: RequestCtx {
                principal: &principal,
                action,
                object: &object,
            }
        })
        .send()
        .await?;

    let result: OPAResponse = reply.json().await?;

    if result.result {
        debug!("authorized", {
            principal: as_debug(&principal),
            action: as_debug(&action),
            object: as_debug(&object)
        });
    } else {
        debug!("unauthorized", {
            principal: as_debug(&principal),
            action: as_debug(&action),
            object: as_debug(&object)
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

    pub async fn check(self, req: &highnoon::Request<State>) -> highnoon::Result<()> {
        let principal = derive_principal(req)?;
        let mut object = self.object.expect("authorization object uninitialised");

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


        if authorize(principal, self.action, object).await? {
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
