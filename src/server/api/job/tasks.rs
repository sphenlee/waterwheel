use crate::server::api::types::{Job, Task};
use anyhow::Result;
use sqlx::{Postgres, Transaction};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug)]
enum ReferenceKind {
    Trigger,
    Task,
}

impl FromStr for ReferenceKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trigger" => Ok(ReferenceKind::Trigger),
            "task" => Ok(ReferenceKind::Task),
            _ => Err(anyhow::Error::msg(
                "failed to parse reference kind (expected \"task\" or \"trigger\"",
            )),
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

fn parse_reference(reference: &str) -> Result<Reference> {
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
        Err(anyhow::Error::msg(
            "invalid reference - expected 2, 3, or 4 slash separated parts",
        ))
    }
}

fn resolve_reference(mut reference: Reference, job: &Job) -> Result<Reference> {
    if reference.proj.is_none() {
        reference.proj = Some(job.project.clone());
    }

    if reference.job.is_none() {
        reference.job = Some(job.name.clone());
    }

    Ok(reference)
}

pub async fn create_task(
    txn: &mut Transaction<'_, Postgres>,
    task: &Task,
    job: &Job,
) -> Result<()> {
    let task_id: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id
            FROM task
            WHERE name = $1
            AND job_id = $2",
    )
    .bind(&task.name)
    .bind(&job.uuid)
    .fetch_optional(&mut *txn)
    .await?;

    let threshold = task.threshold.unwrap_or_else(|| {
        if let Some(dep) = &task.depends {
            dep.len() as u32
        } else {
            1
        }
    });

    let task_id = if let Some((id,)) = task_id {
        sqlx::query(
            "UPDATE task
                SET threshold = $1,
                    image = $2,
                    args = $3,
                    env = $4
                WHERE id = $5",
        )
        .bind(threshold)
        .bind(task.docker.as_ref().map(|d| &d.image))
        .bind(task.docker.as_ref().map(|d| &d.args))
        .bind(task.docker.as_ref().map(|d| &d.env))
        .bind(&id)
        .execute(&mut *txn)
        .await?;

        id
    } else {
        let new_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO task(id, name, job_id, threshold, image, args, env)
                VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&new_id)
        .bind(&task.name)
        .bind(&job.uuid)
        .bind(threshold)
        .bind(task.docker.as_ref().map(|d| &d.image))
        .bind(task.docker.as_ref().map(|d| &d.args))
        .bind(task.docker.as_ref().map(|d| &d.env))
        .execute(&mut *txn)
        .await?;

        new_id
    };

    if let Some(depends) = &task.depends {
        for d in depends {
            let reference = parse_reference(d)?;
            let reference = resolve_reference(reference, &job)?;

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
            let reference = resolve_reference(reference, &job)?;

            match reference.kind {
                ReferenceKind::Trigger => {
                    create_trigger_edge(&mut *txn, &task_id, reference).await?
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
) -> Result<()> {
    sqlx::query(
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
        )
        ON CONFLICT DO NOTHING",
    )
    .bind(reference.proj)
    .bind(reference.job)
    .bind(reference.name)
    .bind(task)
    .execute(txn)
    .await?;

    Ok(())
}

async fn create_task_edge(
    txn: &mut Transaction<'_, Postgres>,
    task: &Uuid,
    reference: Reference,
    kind: &str,
) -> Result<()> {
    sqlx::query(
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
        )
        ON CONFLICT DO NOTHING",
    )
    .bind(reference.proj)
    .bind(reference.job)
    .bind(reference.name)
    .bind(task)
    .bind(kind)
    .execute(txn)
    .await?;

    Ok(())
}
