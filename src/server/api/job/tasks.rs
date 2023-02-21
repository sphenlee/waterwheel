use crate::{
    server::api::{
        auth,
        job::reference::{parse_reference, resolve_reference, Reference, ReferenceKind},
        request_ext::RequestExt,
        types::{Job, Task},
        State,
    },
    util::{is_pg_integrity_error, pg_error},
};
use highnoon::{Json, Request, Responder};
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use tracing::debug;
use uuid::Uuid;

pub async fn create_task(
    txn: &mut Transaction<'_, Postgres>,
    task: &Task,
    job: &Job,
) -> highnoon::Result<Uuid> {
    let threshold = task.threshold.unwrap_or({
        if let Some(dep) = &task.depends {
            dep.len() as i32
        } else {
            1
        }
    });

    let retry_delay_secs = task
        .retry
        .as_ref()
        .and_then(|r| r.delay.as_deref())
        .map(|s| humantime::parse_duration(s))
        .transpose()?
        .map(|dur| dur.as_secs() as i32);

    let timeout_secs = task
        .timeout
        .as_ref()
        .map(|s| humantime::parse_duration(s))
        .transpose()?
        .map(|dur| dur.as_secs() as i32);

    let new_id = Uuid::new_v4();

    let (task_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO task(
            id,
            name,
            job_id,
            threshold,
            retry_max_attempts,
            retry_delay_secs,
            timeout_secs,
            image,
            args,
            env
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT(name, job_id)
         DO UPDATE
         SET threshold = $4,
             retry_max_attempts = $5,
             retry_delay_secs = $6,
             timeout_secs = $7,
             image = $8,
             args = $9,
             env = $10
         RETURNING id",
    )
    .bind(new_id)
    .bind(&task.name)
    .bind(job.uuid)
    .bind(threshold)
    .bind(task.retry.as_ref().map(|r| r.max_attempts))
    .bind(retry_delay_secs)
    .bind(timeout_secs)
    .bind(task.docker.as_ref().map(|d| &d.image))
    .bind(task.docker.as_ref().map(|d| &d.args))
    .bind(task.docker.as_ref().map(|d| &d.env))
    .fetch_one(&mut *txn)
    .await?;

    Ok(task_id)
}

pub async fn create_task_edges(
    txn: &mut Transaction<'_, Postgres>,
    task: &Task,
    job: &Job,
) -> highnoon::Result<()> {
    let (task_id,): (Uuid,) = sqlx::query_as(
        "SELECT id
         FROM task
         WHERE job_id = $1
         AND name = $2",
    )
    .bind(job.uuid)
    .bind(&task.name)
    .fetch_one(&mut *txn)
    .await?;

    // remove existing edges
    sqlx::query(
        "DELETE FROM trigger_edge
        WHERE task_id = $1",
    )
    .bind(task_id)
    .execute(&mut *txn)
    .await?;

    sqlx::query(
        "DELETE FROM task_edge
        WHERE child_task_id = $1",
    )
    .bind(task_id)
    .execute(&mut *txn)
    .await?;

    if let Some(depends) = &task.depends {
        for d in depends {
            let reference = parse_reference(d)?;
            let reference = resolve_reference(reference, job);

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
            let reference = resolve_reference(reference, job);

            match reference.kind {
                ReferenceKind::Trigger => {
                    return Err(highnoon::Error::http((
                        highnoon::StatusCode::BAD_REQUEST,
                        "depends_failure cannot reference a trigger since triggers can't fail",
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
        "INSERT INTO trigger_edge(trigger_id, task_id, edge_offset)
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
            $4,
            $5
        )",
    )
    .bind(&reference.proj)
    .bind(&reference.job)
    .bind(&reference.name)
    .bind(task)
    .bind(reference.offset.map(|offset| offset.num_seconds()))
    .execute(txn)
    .await;

    if let Err(e) = pg_error(res)? {
        if is_pg_integrity_error(&e) {
            debug!("pg integrity error: {}", e.message());
            Err(highnoon::Error::http((
                highnoon::StatusCode::BAD_REQUEST,
                format!(
                    "invalid trigger reference (does this trigger exist?): {reference}"
                ),
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
        "INSERT INTO task_edge(parent_task_id, child_task_id, kind, edge_offset)
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
            $5,
            $6
        )",
    )
    .bind(&reference.proj)
    .bind(&reference.job)
    .bind(&reference.name)
    .bind(task)
    .bind(kind)
    .bind(reference.offset.map(|offset| offset.num_seconds()))
    .execute(txn)
    .await;

    if let Err(e) = pg_error(res)? {
        if is_pg_integrity_error(&e) {
            debug!("pg integrity error: {}", e.message());
            Err(highnoon::Error::http((
                highnoon::StatusCode::BAD_REQUEST,
                format!(
                    "invalid task reference (does this task exist?): {reference}"
                ),
            )))
        } else {
            Err(e.into())
        }
    } else {
        Ok(())
    }
}

#[derive(Serialize, sqlx::FromRow)]
struct ListTask {
    task_id: Uuid,
    name: String,
}

pub async fn list_tasks(req: Request<State>) -> highnoon::Result<impl Responder> {
    let job_id: Uuid = req.param("id")?.parse()?;

    auth::list().job(job_id, None).check(&req).await?;

    let tasks: Vec<ListTask> = sqlx::query_as(
        "SELECT
            id AS task_id,
            name
        FROM task
        WHERE job_id = $1
        ORDER BY name
        LIMIT 200",
    )
    .bind(job_id)
    .fetch_all(&req.get_pool())
    .await?;

    // TODO - check for job_id not found

    Ok(Json(tasks))
}
