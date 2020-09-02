use crate::server::api::util::RequestExt;
use crate::server::api::State;
use chrono::{DateTime, Utc};
use hightide::{Json, Responder};
use serde::{Deserialize, Serialize};
use tide::Request;
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow)]
struct Node {
    id: Uuid,
    kind: String,
    name: String,
    state: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
struct Edge {
    from: Uuid,
    to: Uuid,
    kind: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct QueryGraph {
    trigger_datetime: Option<DateTime<Utc>>,
}

pub async fn get_graph(req: Request<State>) -> tide::Result<impl Responder> {
    let job_id = req.param::<Uuid>("id")?;

    let q: QueryGraph = req.query()?;

    let mut nodes = sqlx::query_as::<_, Node>(
        "SELECT
            t.id AS id,
            'task' AS kind,
            t.name AS name,
            (
                SELECT k.state
                FROM token k
                WHERE k.trigger_datetime = $2
                AND task_id = t.id
            ) AS state
        FROM task t
        WHERE t.job_id = $1
        UNION ALL
        SELECT
            g.id AS id,
            'trigger' AS kind,
            g.name AS name,
            NULL AS state
        FROM trigger g
        WHERE g.job_id = $1",
    )
    .bind(&job_id)
    .bind(&q.trigger_datetime)
    .fetch_all(&req.get_pool())
    .await?;

    let edges = sqlx::query_as::<_, Edge>(
        "SELECT DISTINCT
            te.parent_task_id AS \"from\",
            te.child_task_id AS to,
            te.kind AS kind
        FROM task_edge te
        JOIN task t ON (t.id = te.parent_task_id OR t.id = te.child_task_id)
        WHERE t.job_id = $1
        UNION ALL
        SELECT
            ge.trigger_id AS \"from\",
            ge.task_id AS to,
            'trigger' AS kind
        FROM trigger_edge ge
        JOIN task t ON t.id = ge.task_id
        WHERE t.job_id = $1",
    )
    .bind(&job_id)
    .fetch_all(&req.get_pool())
    .await?;

    let extra_nodes = sqlx::query_as::<_, Node>(
        "SELECT
            t.id AS id,
            'xtask' AS kind,
            t.name AS name,
            (
                SELECT k.state
                FROM token k
                WHERE k.trigger_datetime = $2
                AND task_id = t.id
            ) AS state
        FROM task t
        JOIN task_edge te ON t.id = te.parent_task_id
        JOIN task t2 ON t2.id = te.child_task_id
        WHERE t2.job_id = $1
        AND t.job_id != $1
        UNION ALL
        SELECT
            g.id AS id,
            'xtrigger' AS kind,
            g.name AS name,
            NULL AS state
        FROM trigger g
        JOIN trigger_edge ge ON g.id = ge.trigger_id
        JOIN task t2 ON t2.id = ge.task_id
        WHERE t2.job_id = $1
        AND g.job_id != $1",
    )
    .bind(&job_id)
    .bind(&q.trigger_datetime)
    .fetch_all(&req.get_pool())
    .await?;

    nodes.extend(extra_nodes);

    Ok(Json(Graph { nodes, edges }))
}
