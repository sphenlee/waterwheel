use chrono::{DateTime, Utc};
use sqlx::types::Uuid;
use std::fmt;

/// this struct must be ordered in this way to control the derived sort order
/// (sort by the scheduled time first, then id and the logical time)
#[derive(Copy, Clone, Eq, Hash, Debug, Ord, PartialOrd, PartialEq)]
pub struct TriggerTime {
    /// the time at which this trigger should start (the trigger time + offset)
    pub scheduled_datetime: DateTime<Utc>,
    pub trigger_id: Uuid,
    /// the logical time this trigger is running at
    pub trigger_datetime: DateTime<Utc>,
}

impl fmt::Display for TriggerTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<trigger-time {} @ {}>",
            self.trigger_id,
            self.trigger_datetime.to_rfc3339()
        )
    }
}
