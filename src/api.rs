/// API Types - used to parse the YAML file.
/// These get converted into internal types
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Trigger {
    id: String,
    cron: String,
}

#[derive(Deserialize)]
pub enum Action {
    Docker { image: String, args: Vec<String> },
}

#[derive(Deserialize)]
pub struct Task {
    id: String,
    action: Option<Action>,
    depends: Vec<String>,
    depends_failure: Vec<String>, // TODO - better name for this?
    threshold: Option<u32>,
}

#[derive(Deserialize)]
pub struct Job {
    project_id: String,
    job_id: String,

    triggers: Vec<Trigger>,
    tasks: Vec<Task>,
}
