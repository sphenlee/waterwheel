use chrono::{DateTime, Utc};
/// API Types - used to parse the YAML file.
/// These get converted into internal types
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn period_from_string(period: &Option<String>) -> anyhow::Result<Option<u32>> {
    match period {
        Some(ref s) => {
            let secs = humantime::parse_duration(&s)?.as_secs() as u32;
            Ok(Some(secs))
        }
        None => Ok(None),
    }
}

#[derive(Deserialize, Serialize)]
pub struct Job {
    pub uuid: Uuid,
    pub project: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub paused: bool,
    pub triggers: Vec<Trigger>,
    pub tasks: Vec<Task>,
}

#[derive(Deserialize, Serialize)]
pub struct Trigger {
    pub name: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub period: Option<String>,
    pub cron: Option<String>,
    pub offset: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Docker {
    pub image: String,
    pub args: Vec<String>,
    pub env: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize)]
pub struct Task {
    pub name: String,
    pub docker: Option<Docker>,
    pub depends: Option<Vec<String>>,
    pub depends_failure: Option<Vec<String>>, // TODO - better name for this?
    pub threshold: Option<u32>,
}
