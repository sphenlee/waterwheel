/// API Types - used to parse the YAML file.
/// These get converted into internal types
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub id: String,
    pub start: String,
    pub end: Option<String>,
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
    pub id: String,
    pub docker: Option<Docker>,
    pub depends: Option<Vec<String>>,
    pub depends_failure: Option<Vec<String>>, // TODO - better name for this?
    pub threshold: Option<u32>,
}
