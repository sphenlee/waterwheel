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
    pub threshold: Option<i32>,
}

#[cfg(test)]
mod test {
    use super::period_from_string;

    #[test]
    fn test_period_from_string() -> anyhow::Result<()> {
        assert_eq!(period_from_string(None)?, None);

        assert_eq!(period_from_string(Some("1m"))?, Some(60));
        assert_eq!(period_from_string(Some("10m"))?, Some(600));
        assert_eq!(period_from_string(Some("1h"))?, Some(3600));

        assert_eq!(period_from_string(Some("-1m"))?, Some(-60));
        assert_eq!(period_from_string(Some("-10m"))?, Some(-600));
        assert_eq!(period_from_string(Some("-1h"))?, Some(-3600));

        assert_eq!(period_from_string(Some("- 1m"))?, Some(-60));
        Ok(())
    }

    #[test]
    fn test_period_from_string_errors() -> anyhow::Result<()> {
        let res = period_from_string(Some("1"));
        assert_eq!(res.unwrap_err().to_string().as_str(), "time unit needed, for example 1sec or 1ms");

        let res = period_from_string(Some("1x"));
        assert_eq!(res.unwrap_err().to_string().as_str(), "unknown time unit \"x\", \
            supported units: ns, us, ms, sec, min, hours, days, weeks, months, years (and few variations)");

        let res = period_from_string(Some(""));
        assert_eq!(res.unwrap_err().to_string().as_str(), "value was empty");

        // TODO - we should probably accept this by trimming whitespace
        let res = period_from_string(Some(" -1m"));
        assert_eq!(res.unwrap_err().to_string().as_str(), "expected number at 0");

        Ok(())
    }
}