use crate::trigger_time::TriggerTime;
use anyhow::Result;
use async_std::sync::{Arc, Mutex, Sender};
use binary_heap_plus::{BinaryHeap, MinComparator};
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use log::{debug, info, trace};
use sqlx::postgres::PgQueryAs;
use sqlx::types::Uuid;
use sqlx::{Connection, PgPool};

#[derive(sqlx::FromRow, Debug)]
struct Trigger {
    id: Uuid,
    start_datetime: DateTime<Utc>,
    end_datetime: DateTime<Utc>,
    earliest_trigger_datetime: DateTime<Utc>,
    latest_trigger_datetime: DateTime<Utc>,
    period: i64, // in seconds because sqlx doesn't support duration
}

pub struct TriggerState {
    triggers: Vec<Trigger>,
    queue: BinaryHeap<TriggerTime, MinComparator>,
}

impl TriggerState {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            triggers: Vec::new(),
            queue: BinaryHeap::new_min(),
        }))
    }
}

pub async fn process_triggers(pool: PgPool, execute_tx: Sender<Uuid>) -> Result<()> {
    let state = TriggerState::new();

    info!("restoring triggers from database...");

    // first load all triggers from the DB
    let mut cursor = sqlx::query_as::<_, Trigger>(
        "SELECT
            id,
            start_datetime,
            end_datetime,
            earliest_trigger_datetime,
            latest_trigger_datetime,
            period
        FROM trigger
    ",
    )
    .fetch(&pool);

    while let Some(trigger) = cursor.try_next().await? {
        let period = Duration::seconds(trigger.period);

        if trigger.start_datetime < trigger.earliest_trigger_datetime {
            // start date moved backwards
            debug!(
                "{}: start date has moved backwards - {} -> {}",
                trigger.id, trigger.earliest_trigger_datetime, trigger.start_datetime
            );

            let mut next = trigger.start_datetime;
            while next < trigger.earliest_trigger_datetime {
                activate_trigger(
                    pool.clone(),
                    TriggerTime {
                        trigger_id: trigger.id,
                        trigger_datetime: next,
                    },
                    execute_tx.clone(),
                )
                .await?;
                next = next + period;
            }
        }

        // catchup any periods since the last trigger
        let now = Utc::now();
        let mut next = trigger.latest_trigger_datetime + period;
        while next < now && next < trigger.end_datetime {
            activate_trigger(
                pool.clone(),
                TriggerTime {
                    trigger_id: trigger.id,
                    trigger_datetime: next,
                },
                execute_tx.clone(),
            )
            .await?;
            next = next + period;
        }

        if next < trigger.end_datetime {
            // and push one trigger in the future
            trace!("{}: queueing trigger at {}", trigger.id, next);
            state.lock().await.queue.push(TriggerTime {
                trigger_id: trigger.id,
                trigger_datetime: next,
            });
        }

        state.lock().await.triggers.push(trigger);
    }

    info!("done restoring triggers from database");

    loop {
        let next_trigger = state.lock().await.queue.pop();
        let delay = match next_trigger {
            Some(TriggerTime {
                trigger_datetime, ..
            }) => trigger_datetime - Utc::now(),
            None => Duration::seconds(10),
        };

        debug!("sleeping {} until next trigger", delay);
        async_std::task::sleep(delay.to_std()?).await;
    }

    //error!("trigger processor is exiting - should be restarted by main!");

    //Ok(())
}

async fn activate_trigger(
    pool: PgPool,
    trigger_time: TriggerTime,
    execute_tx: Sender<Uuid>,
) -> Result<()> {
    debug!(
        "{}: activating trigger at {}",
        trigger_time.trigger_id, trigger_time.trigger_datetime
    );

    let mut cursor = sqlx::query_as::<_, (Uuid,)>(
        "SELECT
            te.task_id
        FROM trigger_edge te
        WHERE te.trigger_id = $1",
    )
    .bind(trigger_time.trigger_id)
    .fetch(&pool);

    let conn = pool.acquire().await?;
    let mut txn = conn.begin().await?;

    while let Some((task_id,)) = cursor.try_next().await? {
        trace!("adding token to task: {}", task_id);

        sqlx::query(
            "INSERT INTO token(task_id, trigger_datetime, count, state)
            VALUES ($1, $2, 0, 'waiting')
            ON CONFLICT DO NOTHING",
        )
        .bind(task_id)
        .bind(trigger_time.trigger_datetime)
        .execute(&mut txn)
        .await?;

        sqlx::query(
            "UPDATE token
            SET count = count + 1
            WHERE task_id = $1
            AND trigger_datetime = $2",
        )
        .bind(task_id)
        .bind(trigger_time.trigger_datetime)
        .execute(&mut txn)
        .await?;

        execute_tx.send(task_id).await;
    }

    sqlx::query(
        "
        UPDATE trigger
        SET latest_trigger_datetime = GREATEST(latest_trigger_datetime, $2),
            earliest_trigger_datetime = LEAST(earliest_trigger_datetime, $2)
        WHERE id = $1",
    )
    .bind(trigger_time.trigger_id)
    .bind(trigger_time.trigger_datetime)
    .execute(&mut txn)
    .await?;

    txn.commit().await?;

    Ok(())
}
