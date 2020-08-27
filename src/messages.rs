use crate::server::tokens::Token;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskDef {
    pub task_id: String,
    pub trigger_datetime: String,
    pub image: Option<String>,
    pub args: Vec<String>,
    pub env: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskResult {
    pub task_id: String,
    pub trigger_datetime: String,
    pub result: String,
    pub worker_id: Uuid,
}

impl TaskResult {
    pub fn get_token(&self) -> Result<Token> {
        Ok(Token {
            task_id: Uuid::parse_str(&self.task_id)?,
            trigger_datetime: DateTime::parse_from_rfc3339(&self.trigger_datetime)?
                .with_timezone(&Utc),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WorkerHeartbeat {
    pub uuid: Uuid,
    pub addr: String,
    pub last_seen_datetime: DateTime<Utc>,
}
