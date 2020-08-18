use crate::db;
use crate::server::tokens::{increment_token, Token};
use crate::server::trigger_time::TriggerTime;
use anyhow::Result;
use async_std::sync::{Arc, Mutex, Sender};
use async_std::task;
use binary_heap_plus::{BinaryHeap, MinComparator};
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use log::{debug, info, trace};
use sqlx::types::Uuid;
use sqlx::Connection;
use std::collections::HashMap;
use std::time::Duration as StdDuration;

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
    triggers: HashMap<Uuid, Trigger>,
    queue: BinaryHeap<TriggerTime, MinComparator>,
}

impl TriggerState {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            triggers: HashMap::new(),
            queue: BinaryHeap::new_min(),
        }))
    }
}

pub async fn process_triggers(token_tx: Sender<Token>) -> Result<!> {
    let pool = db::get_pool();
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
                    TriggerTime {
                        trigger_id: trigger.id,
                        trigger_datetime: next,
                    },
                    token_tx.clone(),
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
                TriggerTime {
                    trigger_id: trigger.id,
                    trigger_datetime: next,
                },
                token_tx.clone(),
            )
            .await?;
            next = next + period;
        }

        if next < trigger.end_datetime {
            // push one trigger in the future
            trace!("{}: queueing trigger at {}", trigger.id, next);
            state.lock().await.queue.push(TriggerTime {
                trigger_id: trigger.id,
                trigger_datetime: next,
            });
        }

        state.lock().await.triggers.insert(trigger.id, trigger);
    }

    info!("done restoring triggers from database");

    loop {
        trace!("{} triggers in the queue", state.lock().await.queue.len());

        let next_trigger = {
            loop {
                match state.lock().await.queue.pop() {
                    Some(trigger) => break trigger,
                    None => {
                        trace!("no tasks queued - sleeping");
                        task::sleep(StdDuration::from_secs(60)).await
                    }
                }
            }
        };

        {
            let mut state = state.lock().await;

            let trigger = state
                .triggers
                .get(&next_trigger.trigger_id)
                .expect("trigger missing from hashmap");
            let requeue = TriggerTime {
                trigger_id: next_trigger.trigger_id,
                trigger_datetime: next_trigger.trigger_datetime + Duration::seconds(trigger.period),
            };
            trace!(
                "{}: queueing next time: {}",
                requeue.trigger_id,
                requeue.trigger_datetime
            );
            state.queue.push(requeue);
        }

        let delay = next_trigger.trigger_datetime - Utc::now();
        if delay > Duration::zero() {
            debug!(
                "{}: sleeping {} until next trigger",
                next_trigger.trigger_id, delay
            );
            task::sleep(delay.to_std()?).await;
        } else {
            debug!("overslept trigger: {}", delay)
        }

        activate_trigger(next_trigger, token_tx.clone()).await?;
    }
}

async fn activate_trigger(trigger_time: TriggerTime, token_tx: Sender<Token>) -> Result<()> {
    let pool = db::get_pool();

    debug!("activating trigger: {}", trigger_time);

    let mut cursor = sqlx::query_as::<_, (Uuid,)>(
        "SELECT
            te.task_id
        FROM trigger_edge te
        WHERE te.trigger_id = $1",
    )
    .bind(trigger_time.trigger_id)
    .fetch(&pool);

    let mut conn = pool.acquire().await?;
    let mut txn = conn.begin().await?;

    let mut tokens_to_tx = Vec::new();

    while let Some((task_id,)) = cursor.try_next().await? {
        let token = Token {
            task_id,
            trigger_datetime: trigger_time.trigger_datetime,
        };

        increment_token(&mut txn, &token).await?;
        tokens_to_tx.push(token);
    }

    trace!("updating trigger times for {}", trigger_time);
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
    trace!("done activating trigger: {}", trigger_time);

    // after committing the transaction we can tell the token processor to check thresholds
    for token in tokens_to_tx {
        token_tx.send(token).await;
    }

    Ok(())
}
