use crate::{db, postoffice};
use crate::server::tokens::{increment_token, Token, ProcessToken};
use crate::server::trigger_time::TriggerTime;
use anyhow::Result;
use async_std::task;
use binary_heap_plus::BinaryHeap;
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use log::{debug, info, trace};
use sqlx::types::Uuid;
use sqlx::Connection;
use std::time::Duration as StdDuration;

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

// pub struct TriggerState {
//     triggers: HashMap<Uuid, Trigger>,
//     queue: BinaryHeap<TriggerTime, MinComparator>,
// }
//
// impl TriggerState {
//     fn new() -> Arc<Mutex<Self>> {
//         Arc::new(Mutex::new(Self {
//             triggers: HashMap::new(),
//             queue: BinaryHeap::new_min(),
//         }))
//     }
// }

pub async fn process_triggers() -> Result<!> {
    let pool = db::get_pool();

    let mut queue = BinaryHeap::new_min();

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
            trace!("{}: queueing trigger at {}", trigger.id, next);
            queue.push(trigger.at(next));
        }
    }

    info!("done restoring triggers from database");

    loop {
        while queue.is_empty() {
            debug!("no triggers queued, sleeping for 1m");
            task::sleep(StdDuration::from_secs(60)).await;
        }

        let next_triggertime = queue.pop().expect("queue shouldn't be empty now");

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

            trace!(
                "{}: queueing next time: {}",
                requeue.trigger_id,
                requeue.trigger_datetime
            );

            queue.push(requeue);
        }

        let delay = next_triggertime.trigger_datetime - Utc::now();
        if delay > Duration::zero() {
            debug!(
                "{}: sleeping {} until next trigger",
                next_triggertime.trigger_id, delay
            );
            task::sleep(delay.to_std()?).await;
        } else {
            debug!("overslept trigger: {}", delay)
        }

        activate_trigger(next_triggertime).await?;
    }
}

async fn activate_trigger(trigger_time: TriggerTime) -> Result<()> {
    let pool = db::get_pool();

    let token_tx = postoffice::post_mail::<ProcessToken>().await?;

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
        token_tx.send(ProcessToken(token)).await;
    }

    Ok(())
}

// returns the next trigger time in the future
async fn catchup_trigger(
    trigger: &Trigger
) -> anyhow::Result<DateTime<Utc>> {
    if let Some(earliest) = trigger.earliest_trigger_datetime {
        if trigger.start_datetime < earliest {
            // start date moved backwards
            debug!(
                "{}: start date has moved backwards - {} -> {}",
                trigger.id, earliest, trigger.start_datetime
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
