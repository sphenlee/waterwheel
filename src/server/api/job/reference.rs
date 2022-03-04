use crate::server::api::types::Job;
use chrono::Duration;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    fmt::{self, Display},
    str::FromStr,
};

#[derive(Debug, PartialEq)]
pub enum ReferenceKind {
    Trigger,
    Task,
}

impl FromStr for ReferenceKind {
    type Err = highnoon::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trigger" => Ok(ReferenceKind::Trigger),
            "task" => Ok(ReferenceKind::Task),
            _ => Err(highnoon::Error::http((
                highnoon::StatusCode::BAD_REQUEST,
                format!(
                    "failed to parse reference kind (expected \"task\" \
                         or \"trigger\", got \"{}\")",
                    s
                ),
            ))),
        }
    }
}

impl Display for ReferenceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceKind::Trigger => write!(f, "trigger"),
            ReferenceKind::Task => write!(f, "task"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Reference {
    pub proj: Option<String>,
    pub job: Option<String>,
    pub kind: ReferenceKind,
    pub name: String,
    pub offset: Option<Duration>,
}

impl Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = &self.proj {
            write!(f, "{}/", p)?;
        }
        if let Some(j) = &self.job {
            write!(f, "{}/", j)?;
        }
        write!(f, "{}/{}", self.kind, self.name)?;
        if let Some(offset) = self.offset {
            let offset = offset
                .to_std()
                .expect("overflow converting chrono::Duration to std");
            write!(f, "@{}", humantime::format_duration(offset))?;
        }
        Ok(())
    }
}

static REFERENCE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        "\
        ^\
        ([\\w\\s]+/)?\
        ([\\w\\s]+/)?\
        (trigger|task)/\
        ([\\w\\s]+)\
        (@.+)?\
        $",
    )
    .expect("error compiling regex")
});

pub fn parse_reference(reference: &str) -> highnoon::Result<Reference> {
    let captures = REFERENCE_PATTERN.captures(reference).ok_or_else(|| {
        highnoon::Error::bad_request(format!("invalid reference \'{}\'", reference))
    })?;

    let mut proj = captures
        .get(1)
        .map(|c| c.as_str().trim_end_matches('/').to_owned());
    let mut job = captures
        .get(2)
        .map(|c| c.as_str().trim_end_matches('/').to_owned());

    if proj.is_some() && job.is_none() {
        // TODO - for a 3 part reference, the regex captures the first part as project and leaves job unmatched
        std::mem::swap(&mut proj, &mut job);
    }

    let kind = captures
        .get(3)
        .expect("regex match is missing mandatory capture")
        .as_str()
        .parse()?;
    let name = captures
        .get(4)
        .expect("regex match is missing mandatory capture")
        .as_str()
        .to_owned();
    let offset = captures
        .get(5)
        .map(|c| {
            let offset = humantime::parse_duration(&c.as_str()[1..])?;
            let offset = Duration::from_std(offset)?;
            Ok::<_, anyhow::Error>(offset)
        })
        .transpose()?;

    Ok(Reference {
        proj,
        job,
        kind,
        name,
        offset,
    })
}

pub fn resolve_reference(mut reference: Reference, job: &Job) -> Reference {
    if reference.proj.is_none() {
        reference.proj = Some(job.project.clone());
    }

    if reference.job.is_none() {
        reference.job = Some(job.name.clone());
    }

    reference
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::Duration;
    use std::assert_matches::assert_matches;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_reference() {
        let r = parse_reference("a/b/task/c@1d").unwrap();
        assert_eq!(
            r,
            Reference {
                proj: Some("a".to_owned()),
                job: Some("b".to_owned()),
                kind: ReferenceKind::Task,
                name: "c".to_string(),
                offset: Some(Duration::days(1))
            }
        );

        let r = parse_reference("b/task/c@1d").unwrap();
        assert_eq!(
            r,
            Reference {
                proj: None,
                job: Some("b".to_owned()),
                kind: ReferenceKind::Task,
                name: "c".to_string(),
                offset: Some(Duration::days(1))
            }
        );

        let r = parse_reference("task/c@1d").unwrap();
        assert_eq!(
            r,
            Reference {
                proj: None,
                job: None,
                kind: ReferenceKind::Task,
                name: "c".to_string(),
                offset: Some(Duration::days(1))
            }
        );

        let r = parse_reference("trigger/c@1d").unwrap();
        assert_eq!(
            r,
            Reference {
                proj: None,
                job: None,
                kind: ReferenceKind::Trigger,
                name: "c".to_string(),
                offset: Some(Duration::days(1))
            }
        );

        let r = parse_reference("a/b/task/c").unwrap();
        assert_eq!(
            r,
            Reference {
                proj: Some("a".to_owned()),
                job: Some("b".to_owned()),
                kind: ReferenceKind::Task,
                name: "c".to_string(),
                offset: None
            }
        );

        let r = parse_reference("task/c").unwrap();
        assert_eq!(
            r,
            Reference {
                proj: None,
                job: None,
                kind: ReferenceKind::Task,
                name: "c".to_string(),
                offset: None
            }
        );

        let r = parse_reference("a space/b/task/c").unwrap();
        assert_eq!(
            r,
            Reference {
                proj: Some("a space".to_owned()),
                job: Some("b".to_owned()),
                kind: ReferenceKind::Task,
                name: "c".to_string(),
                offset: None
            }
        );
    }

    #[test]
    fn test_parse_reference_errors() {
        // empty project name
        assert_matches!(parse_reference("/b/task/c"), Err(_));
        // empty job name
        assert_matches!(parse_reference("a//task/c"), Err(_));
        // missing kind
        assert_matches!(parse_reference("a/b//c"), Err(_));
        // invalid kind
        assert_matches!(parse_reference("token/c"), Err(_));
        // empty  name
        assert_matches!(parse_reference("task/"), Err(_));
        // empty string
        assert_matches!(parse_reference(""), Err(_));
        // extra parts
        assert_matches!(parse_reference("a/b/task/c/d"), Err(_));
        // invalid time
        assert_matches!(parse_reference("task/c@not_time"), Err(_));
        // non word char in parts
        assert_matches!(parse_reference("a!/b/task/c"), Err(_));
        assert_matches!(parse_reference("a/b!/task/c"), Err(_));
        assert_matches!(parse_reference("a/b/task/c!"), Err(_));
        assert_matches!(parse_reference("a/b/task/c@!1d"), Err(_));
    }

    #[test]
    fn test_parse_offsets() {
        assert_eq!(
            parse_reference("task/c@1m").unwrap().offset,
            Some(Duration::minutes(1))
        );
        assert_eq!(
            parse_reference("task/c@2m").unwrap().offset,
            Some(Duration::minutes(2))
        );
        assert_eq!(
            parse_reference("task/c@30m").unwrap().offset,
            Some(Duration::minutes(30))
        );

        assert_eq!(
            parse_reference("task/c@30 minutes").unwrap().offset,
            Some(Duration::minutes(30))
        );
        assert_eq!(
            parse_reference("task/c@30minutes").unwrap().offset,
            Some(Duration::minutes(30))
        );
        assert_eq!(
            parse_reference("task/c@30min").unwrap().offset,
            Some(Duration::minutes(30))
        );

        assert_eq!(
            parse_reference("task/c@1h").unwrap().offset,
            Some(Duration::hours(1))
        );
        assert_eq!(
            parse_reference("task/c@2h").unwrap().offset,
            Some(Duration::hours(2))
        );
        assert_eq!(
            parse_reference("task/c@24h").unwrap().offset,
            Some(Duration::days(1))
        );

        assert_eq!(
            parse_reference("task/c@1 h").unwrap().offset,
            Some(Duration::hours(1))
        );
        assert_eq!(
            parse_reference("task/c@1hour").unwrap().offset,
            Some(Duration::hours(1))
        );
        assert_eq!(
            parse_reference("task/c@1hours").unwrap().offset,
            Some(Duration::hours(1))
        );
        assert_eq!(
            parse_reference("task/c@1 hour").unwrap().offset,
            Some(Duration::hours(1))
        );
        assert_eq!(
            parse_reference("task/c@1 hours").unwrap().offset,
            Some(Duration::hours(1))
        );

        assert_eq!(
            parse_reference("task/c@1d").unwrap().offset,
            Some(Duration::days(1))
        );
        assert_eq!(
            parse_reference("task/c@2d").unwrap().offset,
            Some(Duration::days(2))
        );
        assert_eq!(
            parse_reference("task/c@1day").unwrap().offset,
            Some(Duration::days(1))
        );
        assert_eq!(
            parse_reference("task/c@1 day").unwrap().offset,
            Some(Duration::days(1))
        );
        assert_eq!(
            parse_reference("task/c@2days").unwrap().offset,
            Some(Duration::days(2))
        );
        assert_eq!(
            parse_reference("task/c@2 days").unwrap().offset,
            Some(Duration::days(2))
        );

        assert_eq!(
            parse_reference("task/c@1h30m").unwrap().offset,
            Some(Duration::minutes(90))
        );
        assert_eq!(
            parse_reference("task/c@1h 30m").unwrap().offset,
            Some(Duration::minutes(90))
        );
        assert_eq!(
            parse_reference("task/c@1 hour 30 minutes").unwrap().offset,
            Some(Duration::minutes(90))
        );
    }
}
