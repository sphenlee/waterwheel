use chrono::{DateTime, Utc};
/// API Types - used to parse the YAML file.
/// These get converted into internal types
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn period_from_string(period: Option<&str>) -> anyhow::Result<Option<i32>> {
    match period {
        Some(mut s) => {
            let mut neg = false;
            if s.starts_with('-') {
                neg = true;
                s = s.trim_start_matches("-");
            }
            let mut secs = humantime::parse_duration(s)?.as_secs() as i32;
            if neg {
                secs = -secs;
            }
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
    pub paused: Option<bool>,
    pub triggers: Vec<Trigger>,
    pub tasks: Vec<Task>,
}

#[derive(Copy, Clone, Debug, PartialEq, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR")]
pub enum Catchup {
    None,
    Earliest,
    Latest,
    Random,
}

impl Default for Catchup {
    fn default() -> Self {
        Catchup::Earliest
    }
}

#[derive(Deserialize, Serialize)]
pub struct Trigger {
    pub name: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub period: Option<String>,
    pub cron: Option<String>,
    pub offset: Option<String>,
    pub catchup: Option<Catchup>,
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
