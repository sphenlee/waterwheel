use crate::server::api::types::{Job, Task};
use anyhow::Result;
use sqlx::{Postgres, Transaction};
use std::str::FromStr;
use uuid::Uuid;
use crate::util::{is_pg_integrity_error, pg_error};
use log::debug;
use std::fmt::{self, Display};

#[derive(Debug)]
enum ReferenceKind {
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
                format!("failed to parse reference kind (expected \"task\" \
                         or \"trigger\", got \"{}\")", s),
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

#[derive(Debug)]
struct Reference {
    proj: Option<String>,
    job: Option<String>,
    kind: ReferenceKind,
    name: String,
}

impl Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = &self.proj {
            write!(f, "{}/", p)?;
        }
        if let Some(j) = &self.job {
            write!(f, "{}/", j)?;
        }
        write!(f, "{}/{}", self.kind, self.name)
    }
}

fn parse_reference(reference: &str) -> highnoon::Result<Reference> {
    let parts = reference.split('/').collect::<Vec<_>>();

    if parts.len() == 4 {
        Ok(Reference {
            proj: Some(parts[0].to_owned()),
            job: Some(parts[1].to_owned()),
            kind: parts[2].parse()?,
            name: parts[3].to_owned(),
        })
    } else if parts.len() == 3 {
        Ok(Reference {
            proj: None,
            job: Some(parts[0].to_owned()),
            kind: parts[1].parse()?,
            name: parts[2].to_owned(),
        })
    } else if parts.len() == 2 {
        Ok(Reference {
            proj: None,
            job: None,
            kind: parts[0].parse()?,
            name: parts[1].to_owned(),
        })
    } else {
        Err(highnoon::Error::http((
            highnoon::StatusCode::BAD_REQUEST,
            "invalid reference - expected 2, 3, or 4 slash separated parts",
        )))
    }
}

fn resolve_reference(mut reference: Reference, job: &Job) -> Reference {
    if reference.proj.is_none() {
        reference.proj = Some(job.project.clone());
    }

    if reference.job.is_none() {
        reference.job = Some(job.name.clone());
    }

    reference
}

pub async fn create_task(
    txn: &mut Transaction<'_, Postgres>,
    task: &Task,
    job: &Job,
) -> highnoon::Result<()> {
    let threshold = task.threshold.unwrap_or_else(|| {
        if let Some(dep) = &task.depends {
            dep.len() as u32
        } else {
            1
        }
    });

    let new_id = Uuid::new_v4();

    let (task_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO task(id, name, job_id, threshold, image, args, env)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT(name, job_id)
         DO UPDATE
         SET threshold = $4,
             image = $5,
             args = $6,
             env = $7
         RETURNING id",
    )
    .bind(&new_id)
    .bind(&task.name)
    .bind(&job.uuid)
    .bind(threshold)
    .bind(task.docker.as_ref().map(|d| &d.image))
    .bind(task.docker.as_ref().map(|d| &d.args))
    .bind(task.docker.as_ref().map(|d| &d.env))
    .fetch_one(&mut *txn)
    .await?;

    // remove existing edges
    sqlx::query(
        "DELETE FROM trigger_edge
        WHERE task_id = $1",
    )
    .bind(&task_id)
    .execute(&mut *txn)
    .await?;

    sqlx::query(
        "DELETE FROM task_edge
        WHERE child_task_id = $1",
    )
    .bind(&task_id)
    .execute(&mut *txn)
    .await?;

    if let Some(depends) = &task.depends {
        for d in depends {
            let reference = parse_reference(d)?;
            let reference = resolve_reference(reference, &job);

            match reference.kind {
                ReferenceKind::Trigger => {
                    create_trigger_edge(&mut *txn, &task_id, reference).await?
                }
                ReferenceKind::Task => {
                    create_task_edge(&mut *txn, &task_id, reference, "success").await?
                }
            }
        }
    }

    // TODO refactor this duplicate code
    if let Some(depends) = &task.depends_failure {
        for d in depends {
            let reference = parse_reference(d)?;
            let reference = resolve_reference(reference, &job);

            match reference.kind {
                ReferenceKind::Trigger => {
                    return Err(highnoon::Error::http((
                        highnoon::StatusCode::BAD_REQUEST,
                        "depends_failure cannot reference a trigger since triggers can't fail"
                        )));
                }
                ReferenceKind::Task => {
                    create_task_edge(&mut *txn, &task_id, reference, "failure").await?
                }
            }
        }
    }

    Ok(())
}

async fn create_trigger_edge(
    txn: &mut Transaction<'_, Postgres>,
    task: &Uuid,
    reference: Reference,
) -> highnoon::Result<()> {
    let res = sqlx::query(
        "INSERT INTO trigger_edge(trigger_id, task_id)
        VALUES(
            (
                SELECT t.id
                FROM trigger t
                JOIN job j ON j.id = t.job_id
                JOIN project p ON p.id = j.project_id
                WHERE p.name = $1
                AND j.name = $2
                AND t.name = $3
            ),
            $4
        )",
    )
    .bind(&reference.proj)
    .bind(&reference.job)
    .bind(&reference.name)
    .bind(task)
    .execute(txn)
    .await;

    if let Err(e) = pg_error(res)? {
        if is_pg_integrity_error(&e) {
            debug!("pg integrity error: {}", e.message());
            Err(highnoon::Error::http((
                highnoon::StatusCode::BAD_REQUEST,
                format!("invalid trigger reference (does this trigger exist?): {}", reference),
            )))
        } else {
            Err(e.into())
        }
    } else {
        Ok(())
    }
}

async fn create_task_edge(
    txn: &mut Transaction<'_, Postgres>,
    task: &Uuid,
    reference: Reference,
    kind: &str,
) -> highnoon::Result<()> {
    let res = sqlx::query(
        "INSERT INTO task_edge(parent_task_id, child_task_id, kind)
        VALUES(
            (
                SELECT t.id
                FROM task t
                JOIN job j ON j.id = t.job_id
                JOIN project p ON p.id = j.project_id
                WHERE p.name = $1
                AND j.name = $2
                AND t.name = $3
            ),
            $4,
            $5
        )",
    )
    .bind(&reference.proj)
    .bind(&reference.job)
    .bind(&reference.name)
    .bind(task)
    .bind(kind)
    .execute(txn)
    .await;

    if let Err(e) = pg_error(res)? {
        if is_pg_integrity_error(&e) {
            debug!("pg integrity error: {}", e.message());
            Err(highnoon::Error::http((
                highnoon::StatusCode::BAD_REQUEST,
                format!("invalid task reference (does this task exist?): {}", reference),
            )))
        } else {
            Err(e.into())
        }
    } else {
        Ok(())
    }
}
