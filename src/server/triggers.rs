use crate::messages::{TaskPriority, Token};
use crate::server::status::SERVER_STATUS;
use crate::server::tokens::{increment_token, ProcessToken};
use crate::server::trigger_time::TriggerTime;
use crate::{db, postoffice};
use anyhow::Result;
use binary_heap_plus::{BinaryHeap, MinComparator};
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use futures::future::{self, Either};
use futures::TryStreamExt;
use kv_log_macro::{debug, info, trace, warn};
use postage::prelude::*;
use sqlx::types::Uuid;
use sqlx::Connection;
use std::str::FromStr;
use tokio::time;
use postage::stream::TryRecvError;

type Queue = BinaryHeap<TriggerTime, MinComparator>;

#[derive(Clone, Debug)]
pub struct TriggerUpdate(pub Uuid);

#[derive(sqlx::FromRow, Debug)]
struct Trigger {
    id: Uuid,
    start_datetime: DateTime<Utc>,
    end_datetime: Option<DateTime<Utc>>,
    earliest_trigger_datetime: Option<DateTime<Utc>>,
    latest_trigger_datetime: Option<DateTime<Utc>>,
    period: Option<i64>, // in seconds because sqlx doesn't support duration
    cron: Option<String>,
    trigger_offset: Option<i64>,
}

enum Period {
    Duration(Duration),
    Cron(Schedule),
}

impl std::ops::Add<&Period> for DateTime<Utc> {
    type Output = Self;

    fn add(self, rhs: &Period) -> Self::Output {
        match rhs {
            Period::Duration(duration) => self + *duration,
            Period::Cron(schedule) => schedule.after(&self).next().unwrap(),
        }
    }
}

impl Trigger {
    fn period(&self) -> Result<Period> {
        Ok(if let Some(ref cron) = self.cron {
            Period::Cron(
                Schedule::from_str(&cron).map_err(|err| anyhow::Error::msg(err.to_string()))?,
            )
        } else {
            Period::Duration(Duration::seconds(self.period.unwrap()))
        })
    }

    fn at(&self, datetime: DateTime<Utc>) -> TriggerTime {
        TriggerTime {
            trigger_datetime: datetime,
            trigger_id: self.id,
        }
    }
}

pub async fn process_triggers() -> Result<!> {
    let mut trigger_rx = postoffice::receive_mail::<TriggerUpdate>().await?;
    let mut queue = Queue::new_min();

    restore_triggers(&mut queue).await?;

    loop {
        trace!("checking for pending trigger updates");
        loop {
            match trigger_rx.try_recv() {
                Ok(TriggerUpdate(uuid)) => {
                    // TODO - batch the updates to avoid multiple heap recreations
                    update_trigger(&uuid, &mut queue).await?;
                },
                Err(TryRecvError::Pending) => break,
                Err(TryRecvError::Closed) => panic!("TriggerUpdated channel was closed!")
            }
        }
        trace!("no trigger updates pending - going around the scheduler loop again");

        // rather than update this every place we edit the queue just do it
        // once per loop - it's for monitoring purposes anyway
        SERVER_STATUS.lock().await.queued_triggers = queue.len();

        if queue.is_empty() {
            debug!("no triggers queued, waiting for a trigger update");
            let TriggerUpdate(uuid) = trigger_rx
                .recv()
                .await
                .expect("TriggerUpdate channel was closed!");
            update_trigger(&uuid, &mut queue).await?;
            continue;
        }

        #[cfg(debug_assertions)]
        if log::max_level() >= log::Level::Trace {
            let queue_copy = queue.clone();
            trace!("dumping the first 10 triggers in the queue:");
            for trigger in queue_copy.into_iter_sorted().take(10) {
                trace!(
                    "    {}: {}",
                    trigger.trigger_datetime.to_rfc3339(),
                    trigger.trigger_id
                );
            }
        }

        let next_triggertime = queue.pop().expect("queue shouldn't be empty now");

        let delay = next_triggertime.trigger_datetime - Utc::now();
        if delay > Duration::zero() {
            debug!("sleeping {} until next trigger", delay, {
                trigger_id: next_triggertime.trigger_id.to_string()
            });

            let sleep = Box::pin(time::sleep(delay.to_std()?));
            let recv = Box::pin(trigger_rx.recv());

            match future::select(recv, sleep).await {
                Either::Left((recv, _)) => {
                    trace!("received a trigger update while sleeping");
                    let TriggerUpdate(uuid) = recv.expect("TriggerUpdate channel was closed!");

                    // put the trigger we slept on back in the queue
                    // update trigger might delete it, or we might select it as the next trigger
                    queue.push(next_triggertime);

                    update_trigger(&uuid, &mut queue).await?;
                }
                Either::Right((_, _)) => {
                    trace!("sleep completed, no updates");
                    requeue_next_triggertime(&next_triggertime, &mut queue).await?;
                    activate_trigger(next_triggertime, TaskPriority::Normal).await?;
                }
            };
        } else {
            warn!("overslept trigger: {}", delay);
            requeue_next_triggertime(&next_triggertime, &mut queue).await?;
            activate_trigger(next_triggertime, TaskPriority::Normal).await?;
        }
    }
}

async fn activate_trigger(trigger_time: TriggerTime, priority: TaskPriority) -> Result<()> {
    let pool = db::get_pool();

    let mut token_tx = postoffice::post_mail::<ProcessToken>().await?;

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
        token_tx
            .send(ProcessToken::Increment(token, priority))
            .await?;
    }

    Ok(())
}

// returns the next trigger time in the future
async fn catchup_trigger(trigger: &Trigger) -> anyhow::Result<DateTime<Utc>> {
    let period = trigger.period()?;

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
                activate_trigger(trigger.at(next), TaskPriority::BackFill).await?;
                next = next + &period;
            }
        }
    }

    // catchup any periods since the last trigger
    let now = Utc::now();

    let mut next = if let Some(latest) = trigger.latest_trigger_datetime {
        latest + &period
    } else {
        trigger.start_datetime
    };

    let last = if let Some(end) = trigger.end_datetime {
        std::cmp::min(now, end)
    } else {
        now
    };

    while next < last {
        activate_trigger(trigger.at(next), TaskPriority::BackFill).await?;
        next = next + &period;
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
    let maybe_trigger: Option<Trigger> = sqlx::query_as(
        "SELECT
            t.id AS id,
            start_datetime,
            end_datetime,
            earliest_trigger_datetime,
            latest_trigger_datetime,
            period,
            cron,
            trigger_offset
        FROM trigger t
        JOIN job j ON t.job_id = j.id
        WHERE t.id = $1
        AND NOT j.paused
    ",
    )
    .bind(uuid)
    .fetch_optional(&pool)
    .await?;

    if let Some(trigger) = maybe_trigger {
        // do a catchup
        let next = catchup_trigger(&trigger).await?;

        if trigger.end_datetime.is_none() || next < trigger.end_datetime.unwrap() {
            // push one trigger in the future
            trace!("queueing trigger at {}", next, { trigger_id: trigger.id.to_string() });
            queue.push(trigger.at(next));
        }
    } else {
        debug!(
            "trigger {} has been paused, it had been removed from the queue",
            uuid
        );
    }

    Ok(())
}

async fn restore_triggers(queue: &mut Queue) -> Result<()> {
    let pool = db::get_pool();

    info!("restoring triggers from database...");

    // first load all unpaused triggers from the DB
    let mut cursor = sqlx::query_as::<_, Trigger>(
        "SELECT
            t.id AS id,
            start_datetime,
            end_datetime,
            earliest_trigger_datetime,
            latest_trigger_datetime,
            period,
            cron,
            trigger_offset
        FROM trigger t
        JOIN job j ON t.job_id = j.id
        WHERE NOT j.paused
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

    info!("done restoring {} triggers from database", queue.len());

    Ok(())
}

async fn requeue_next_triggertime(next_triggertime: &TriggerTime, queue: &mut Queue) -> Result<()> {
    let pool = db::get_pool();

    // get the trigger's info from the DB
    let trigger: Trigger = sqlx::query_as(
        "SELECT
            id,
            start_datetime,
            end_datetime,
            earliest_trigger_datetime,
            latest_trigger_datetime,
            period,
            cron,
            trigger_offset
        FROM trigger
        WHERE id = $1
    ",
    )
    .bind(&next_triggertime.trigger_id)
    .fetch_one(&pool)
    .await?;

    let next_datetime = next_triggertime.trigger_datetime + &trigger.period()?;

    if trigger.end_datetime.is_none() || next_datetime < trigger.end_datetime.unwrap() {
        let requeue = trigger.at(next_datetime);

        trace!("queueing next time: {}", requeue.trigger_datetime, {
            trigger_id: requeue.trigger_id.to_string()
        });

        queue.push(requeue);
    }

    Ok(())
}
