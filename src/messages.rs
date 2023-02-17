use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::error;
use uuid::Uuid;

/// state of a token
// TODO - strings are still hardcoded, use the enum!
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR")]
pub enum TokenState {
    /// waiting for the count to reach the threshold
    Waiting,
    /// task was sent for execution but the job was paused
    Cancelled,
    /// task has been sent to the message broker to be started
    Active,
    /// running the task
    Running,
    /// task completed successfully
    Success,
    /// task failed
    Failure,
    /// an error occurred (ie. task did not succeed or fail)
    Error,
    /// task failed but is going to be retried
    Retry,
}

impl TokenState {
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            TokenState::Success | TokenState::Failure | TokenState::Error
        )
    }

    pub fn from_result(result: Result<bool>) -> Self {
        match result {
            Ok(true) => TokenState::Success,
            Ok(false) => TokenState::Failure,
            Err(err) => {
                error!("failed to run task: {:#}", err);
                TokenState::Error
            }
        }
    }
}

impl AsRef<str> for TokenState {
    fn as_ref(&self) -> &str {
        match self {
            TokenState::Waiting => "waiting",
            TokenState::Active => "active",
            TokenState::Running => "running",
            TokenState::Success => "success",
            TokenState::Failure => "failure",
            TokenState::Error => "error",
            TokenState::Cancelled => "cancelled",
            TokenState::Retry => "retry",
        }
    }
}

pub struct TokenStateParseError(pub String);

impl FromStr for TokenState {
    type Err = TokenStateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "waiting" => Ok(TokenState::Waiting),
            "active" => Ok(TokenState::Active),
            "running" => Ok(TokenState::Running),
            "success" => Ok(TokenState::Success),
            "failure" => Ok(TokenState::Failure),
            "error" => Ok(TokenState::Error),
            "cancelled" => Ok(TokenState::Cancelled),
            "retry" => Ok(TokenState::Retry),
            _ => Err(TokenStateParseError(format!(
                "invalid token state: '{s}'"
            ))),
        }
    }
}

#[derive(PartialEq, Hash, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub task_id: Uuid,
    pub trigger_datetime: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskRequest {
    pub task_run_id: Uuid,
    pub task_id: Uuid,
    pub trigger_datetime: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct TaskDef {
    pub task_id: Uuid,
    pub task_name: String,
    pub job_id: Uuid,
    pub job_name: String,
    pub project_id: Uuid,
    pub project_name: String,
    pub image: Option<String>,
    pub args: Vec<String>,
    pub env: Option<Vec<String>>,
    pub paused: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskProgress {
    pub task_run_id: Uuid,
    pub task_id: Uuid,
    pub trigger_datetime: DateTime<Utc>,
    pub started_datetime: DateTime<Utc>,
    pub finished_datetime: Option<DateTime<Utc>>,
    pub result: TokenState,
    pub worker_id: Uuid,
}

// impl TaskProgress {
//     pub fn get_token(&self) -> Result<Token> {
//         Ok(Token {
//             task_id: self.task_id.clone(),
//             trigger_datetime: self.trigger_datetime.clone(),
//         })
//     }
// }

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Serialize, Deserialize, sqlx::Type,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR")]
pub enum TaskPriority {
    BackFill = 0,
    Low = 1,
    Normal = 2,
    High = 3,
}

impl TaskPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskPriority::BackFill => "backfill",
            TaskPriority::Low => "low",
            TaskPriority::Normal => "normal",
            TaskPriority::High => "high",
        }
    }
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WorkerHeartbeat {
    pub uuid: Uuid,
    pub addr: String,
    pub last_seen_datetime: DateTime<Utc>,
    pub running_tasks: i32,
    pub total_tasks: i32,
    pub version: String,
}

/// Message sent from API to scheduler to notify of a trigger being updated.
/// The changes made have already been committed to the database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerUpdate(pub Vec<Uuid>);

/// Message sent from API to scheduler to perform token operations.
/// This message is also sent within the scheduler from other tokio tasks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProcessToken {
    /// Add a token to a task
    Increment(Token, TaskPriority),
    /// Add N to a task, where N is the task's threshold (ie. causes it to activate right now)
    Activate(Token, TaskPriority),
    /// Remove all tokens from a task
    Clear(Token),
    /// Unpause a job, check if any tasks are ready to activate
    UnpauseJob(Uuid),
}

/// message sent from the API to the workers to update config items
#[derive(Serialize, Deserialize, Debug)]
pub enum ConfigUpdate {
    Project(Uuid),
    TaskDef(Uuid),
}
