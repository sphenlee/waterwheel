use crate::config;
use crate::messages::TaskDef;
use crate::server::stash;
use anyhow::Result;
use itertools::Itertools;
use k8s_openapi::api::core::v1::EnvVar;

pub fn get_env_string(task_def: &TaskDef) -> Result<Vec<String>> {
    let env = get_env(task_def)?;

    Ok(env
        .iter()
        .map(|ev| format!("{}={}", ev.name, ev.value.as_ref().unwrap()))
        .collect())
}

fn envvar(name: &str, val: impl std::fmt::Display) -> EnvVar {
    EnvVar {
        name: name.to_owned(),
        value: Some(val.to_string()),
        value_from: None,
    }
}

pub fn get_env(task_def: &TaskDef) -> Result<Vec<EnvVar>> {
    let provided_env = task_def.env.clone().unwrap_or_default();

    let mut env = vec![];

    for kv in provided_env {
        if let Some((k, v)) = kv.splitn(2, '=').collect_tuple() {
            env.push(envvar(k, v));
        } else {
            return Err(anyhow::Error::msg(
                "invalid environment variable (only KEY=VALUE syntax is supported)",
            ));
        }
    }

    let server_addr: String = config::get("WATERWHEEL_SERVER_ADDR")?;

    env.push(envvar(
        "WATERWHEEL_TRIGGER_DATETIME",
        task_def
            .trigger_datetime
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    ));
    env.push(envvar("WATERWHEEL_TASK_NAME", &task_def.task_name));
    env.push(envvar("WATERWHEEL_TASK_ID", task_def.task_id));
    env.push(envvar("WATERWHEEL_JOB_NAME", &task_def.job_name));
    env.push(envvar("WATERWHEEL_JOB_ID", task_def.job_id));
    env.push(envvar("WATERWHEEL_PROJECT_NAME", &task_def.project_name));
    env.push(envvar("WATERWHEEL_PROJECT_ID", task_def.project_id));
    env.push(envvar("WATERWHEEL_SERVER_ADDR", server_addr));

    let stash_jwt = stash::generate_jwt(&task_def.task_id.to_string())?;
    env.push(envvar("WATERWHEEL_JWT", stash_jwt));

    Ok(env)
}
