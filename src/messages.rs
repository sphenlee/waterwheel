use anyhow::Result;
use chrono::{DateTime, Utc, serde::ts_seconds};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// state of a token
// TODO - strings are still hardcoded, use the enum!
pub enum TokenState {
    // waiting for the count to reach the threshold
    Waiting,
    // task has been sent to the message broker to be started
    Active,
    // running the task
    Running,
    // task completed successfully
    Success,
    // tails failed
    Failure,
}

// TODO - move this out into general code
#[derive(PartialEq, Hash, Eq, Clone, Debug)]
pub struct Token {
    pub task_id: Uuid,
    pub trigger_datetime: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskDef {
    pub task_run_id: Uuid,
    pub task_id: Uuid,
    pub task_name: String,
    pub job_id: Uuid,
    pub job_name: String,
    pub project_id: Uuid,
    pub project_name: String,
    pub trigger_datetime: DateTime<Utc>,
    pub image: Option<String>,
    pub args: Vec<String>,
    pub env: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskResult {
    pub task_run_id: Uuid,
    pub task_id: Uuid,
    pub trigger_datetime: DateTime<Utc>,
    pub started_datetime: DateTime<Utc>,
    pub finished_datetime: DateTime<Utc>,
    pub result: String,
    pub worker_id: Uuid,
}

impl TaskResult {
    pub fn get_token(&self) -> Result<Token> {
        Ok(Token {
            task_id: self.task_id.clone(),
            trigger_datetime: self.trigger_datetime.clone(),
        })
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum TaskPriority {
    BackFill = 0,
    Low = 1,
    Normal = 2,
    High = 3,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WorkerHeartbeat {
    pub uuid: Uuid,
    pub addr: String,
    pub last_seen_datetime: DateTime<Utc>,
}
