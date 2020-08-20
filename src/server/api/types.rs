use chrono::{DateTime, Utc};
/// API Types - used to parse the YAML file.
/// These get converted into internal types
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn period_from_string(s: &str) -> anyhow::Result<u32> {
    Ok(humantime::parse_duration(&s)?.as_secs() as u32)
}

#[derive(Deserialize, Serialize)]
pub struct Job {
    pub uuid: Uuid,
    pub project: String,
    pub name: String,
    pub triggers: Vec<Trigger>,
    pub tasks: Vec<Task>,
}

#[derive(Deserialize, Serialize)]
pub struct Trigger {
    pub name: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub period: String,
    pub offset: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Docker {
    image: String,
    args: Vec<String>,
    env: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize)]
pub struct Task {
    pub name: String,
    pub docker: Option<Docker>,
    pub depends: Option<Vec<String>>,
    pub depends_failure: Option<Vec<String>>, // TODO - better name for this?
    pub threshold: Option<u32>,
}
