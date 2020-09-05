use chrono::{DateTime, Utc};
use sqlx::types::Uuid;
use std::fmt;

#[derive(Clone, Eq, Hash, Debug, Ord, PartialOrd, PartialEq)]
pub struct TriggerTime {
    // ensure triggers are sorted by time before id
    pub trigger_datetime: DateTime<Utc>,
    pub trigger_id: Uuid,
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
