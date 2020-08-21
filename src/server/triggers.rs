use crate::server::tokens::{increment_token, ProcessToken, Token};
use crate::server::trigger_time::TriggerTime;
use crate::{db, postoffice};
use anyhow::Result;
use async_std::future::timeout;
use async_std::task;
use binary_heap_plus::{BinaryHeap, MinComparator};
use chrono::{DateTime, Duration, Utc};
use futures::future::{self, Either};
use futures::TryStreamExt;
use kv_log_macro::{debug, info, trace, warn};
use sqlx::types::Uuid;
use sqlx::Connection;

const SMALL_SLEEP: std::time::Duration = std::time::Duration::from_millis(50);

type Queue = BinaryHeap<TriggerTime, MinComparator>;

pub struct TriggerUpdate(pub Uuid);

#[derive(sqlx::FromRow, Debug)]
struct Trigger {
    id: Uuid,
    start_datetime: DateTime<Utc>,
    end_datetime: Option<DateTime<Utc>>,
    earliest_trigger_datetime: Option<DateTime<Utc>>,
    latest_trigger_datetime: Option<DateTime<Utc>>,
    period: i64, // in seconds because sqlx doesn't support duration
}

impl Trigger {
    fn period(&self) -> Duration {
        Duration::seconds(self.period)
    }

    fn at(&self, datetime: DateTime<Utc>) -> TriggerTime {
        TriggerTime {
            trigger_datetime: datetime,
            trigger_id: self.id,
        }
    }
}

pub async fn process_triggers() -> Result<!> {
    let trigger_rx = postoffice::receive_mail::<TriggerUpdate>().await?;
    let mut queue = Queue::new_min();

    restore_triggers(&mut queue).await?;

    loop {
        if queue.is_empty() {
            debug!("no triggers queued, waiting for a trigger update");
            let TriggerUpdate(uuid) = trigger_rx.recv().await?;
            update_trigger(&uuid, &mut queue).await?;
        }

        trace!("checking for pending trigger updates");
        while let Ok(recv) = timeout(SMALL_SLEEP, trigger_rx.recv()).await {
            let TriggerUpdate(uuid) = recv?;
            update_trigger(&uuid, &mut queue).await?;
        }
        trace!("no trigger updates pending - going around the scheduler loop again");

        let next_triggertime = queue.pop().expect("queue shouldn't be empty now");

        let delay = next_triggertime.trigger_datetime - Utc::now();
        if delay > Duration::zero() {
            debug!("sleeping {} until next trigger", delay, {
                trigger_id: next_triggertime.trigger_id.to_string()
            });

            let sleep = Box::pin(task::sleep(delay.to_std()?));
            let recv = Box::pin(trigger_rx.recv());

            match future::select(recv, sleep).await {
                Either::Left((recv, _)) => {
                    trace!("received a trigger update while sleeping");
                    let TriggerUpdate(uuid) = recv?;

                    // put the trigger we slept on back in the queue
                    // update trigger might delete it, or we might select it as the next trigger
                    queue.push(next_triggertime);

                    update_trigger(&uuid, &mut queue).await?;
                }
                Either::Right((_, _)) => {
                    trace!("sleep completed, no updates");
                    requeue_next_triggertime(&next_triggertime, &mut queue).await?;
                    activate_trigger(next_triggertime).await?;
                }
            };
        } else {
            warn!("overslept trigger: {}", delay);
            requeue_next_triggertime(&next_triggertime, &mut queue).await?;
            activate_trigger(next_triggertime).await?;
        }
    }
}

async fn activate_trigger(trigger_time: TriggerTime) -> Result<()> {
    let pool = db::get_pool();

    let token_tx = postoffice::post_mail::<ProcessToken>().await?;

    debug!("activating trigger", {
        trigger_id: trigger_time.trigger_id.to_string(),
        trigger_datetime: trigger_time.trigger_datetime.to_rfc3339(),
    });

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
        token_tx.send(ProcessToken(token)).await;
    }

    Ok(())
}

// returns the next trigger time in the future
async fn catchup_trigger(trigger: &Trigger) -> anyhow::Result<DateTime<Utc>> {
    if let Some(earliest) = trigger.earliest_trigger_datetime {
        if trigger.start_datetime < earliest {
            // start date moved backwards
            debug!(
                "start date has moved backwards: {} -> {}",
                earliest, trigger.start_datetime,
                { trigger_id: trigger.id.to_string() }
            );

            let mut next = trigger.start_datetime;
            while next < earliest {
                activate_trigger(trigger.at(next)).await?;
                next = next + trigger.period();
            }
        }
    }

    // catchup any periods since the last trigger
    let now = Utc::now();

    let mut next = if let Some(latest) = trigger.latest_trigger_datetime {
        latest + trigger.period()
    } else {
        trigger.start_datetime
    };

    let last = if let Some(end) = trigger.end_datetime {
        std::cmp::min(now, end)
    } else {
        now
    };

    while next < last {
        activate_trigger(trigger.at(next)).await?;
        next = next + trigger.period();
    }

    Ok(next)
}

async fn update_trigger(uuid: &Uuid, queue: &mut Queue) -> Result<()> {
    let pool = db::get_pool();

    debug!("updating trigger", { trigger_id: uuid.to_string() });

    // de-heapify the triggers and delete the one we are updating
    let mut triggers = queue
        .drain()
        .filter(|t| t.trigger_id != *uuid)
        .collect::<Vec<_>>();

    // now heapify them again
    queue.extend(triggers.drain(..));

    // get the trigger's new info from the DB
    let trigger: Trigger = sqlx::query_as(
        "SELECT
            id,
            start_datetime,
            end_datetime,
            earliest_trigger_datetime,
            latest_trigger_datetime,
            period
        FROM trigger
        WHERE id = $1
    ",
    )
    .bind(uuid)
    .fetch_one(&pool)
    .await?;

    // do a catchup
    let next = catchup_trigger(&trigger).await?;

    if trigger.end_datetime.is_none() || next < trigger.end_datetime.unwrap() {
        // push one trigger in the future
        trace!("queueing trigger at {}", next, { trigger_id: trigger.id.to_string() });
        queue.push(trigger.at(next));
    }

    Ok(())
}

async fn restore_triggers(queue: &mut Queue) -> Result<()> {
    let pool = db::get_pool();

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
        let next = catchup_trigger(&trigger).await?;

        if trigger.end_datetime.is_none() || next < trigger.end_datetime.unwrap() {
            // push one trigger in the future
            trace!("queueing trigger at {}", next, { trigger_id: trigger.id.to_string() });
            queue.push(trigger.at(next));
        }
    }

    info!("done restoring triggers from database");

    Ok(())
}

async fn requeue_next_triggertime(next_triggertime: &TriggerTime, queue: &mut Queue) -> Result<()> {
    let pool = db::get_pool();

    let (period, end_datetime): (i64, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT period, end_datetime
            FROM trigger
            WHERE id = $1",
    )
    .bind(&next_triggertime.trigger_id)
    .fetch_one(&pool)
    .await?;

    let next_datetime = next_triggertime.trigger_datetime + Duration::seconds(period);

    if end_datetime.is_none() || next_datetime < end_datetime.unwrap() {
        let requeue = TriggerTime {
            trigger_datetime: next_datetime,
            trigger_id: next_triggertime.trigger_id,
        };

        trace!("queueing next time: {}", requeue.trigger_datetime, {
            trigger_id: requeue.trigger_id.to_string()
        });

        queue.push(requeue);
    }

    Ok(())
}
