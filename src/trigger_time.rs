use chrono::{DateTime, Utc};
use sqlx::types::Uuid;
use std::cmp::Ordering;
use std::fmt;

#[derive(Eq, Hash, Debug)]
pub struct TriggerTime {
    pub trigger_id: Uuid,
    pub trigger_datetime: DateTime<Utc>,
}

impl PartialOrd for TriggerTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TriggerTime {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Ord for TriggerTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.trigger_datetime.cmp(&other.trigger_datetime)
    }
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
