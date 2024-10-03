use crate::{
    messages::{ProcessToken, TaskPriority, Token},
    server::{api::types::Catchup, tokens::increment_token, trigger_time::TriggerTime, Server},
    util::format_duration_approx,
};
use anyhow::Result;
use binary_heap_plus::{BinaryHeap, MinComparator};
use cadence::Gauged;
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use futures::TryStreamExt;
use postage::{prelude::*, stream::TryRecvError};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, PgPool, Postgres, Transaction};
use std::{
    str::FromStr,
    sync::{atomic::Ordering, Arc},
};
use std::collections::HashSet;
use tokio::time;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;
use crate::messages::TriggerUpdate;
use crate::util::{deref, first};

type Queue = BinaryHeap<TriggerTime, MinComparator>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TriggerChange {
    Add(Vec<Uuid>),
    Remove(Vec<Uuid>),
}

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
    catchup: Catchup,
}

enum Period {
    Duration(Duration),
    Cron(Box<Schedule>),
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
            Period::Cron(Box::new(Schedule::from_str(cron)?))
        } else {
            Period::Duration(Duration::seconds(self.period.unwrap()))
        })
    }

    fn offset_duration(&self) -> Duration {
        if let Some(offset) = self.trigger_offset {
            Duration::seconds(offset)
        } else {
            Duration::zero()
        }
    }

    fn at(&self, datetime: DateTime<Utc>) -> TriggerTime {
        TriggerTime {
            scheduled_datetime: datetime + self.offset_duration(),
            trigger_datetime: datetime,
            trigger_id: self.id,
        }
    }
}

pub async fn process_triggers(server: Arc<Server>) -> Result<!> {
    let mut trigger_rx = server.post_office.receive_mail::<TriggerChange>().await?;
    let mut queue = Queue::new_min();

    let statsd = server.statsd.clone();

    //restore_triggers(&server, &mut queue).await?;

    loop {
        trace!("checking for pending trigger updates");
        loop {
            match trigger_rx.try_recv() {
                Ok(trigger_change) => {
                    // TODO - batch the updates to avoid multiple heap recreations
                    update_trigger(&server, trigger_change, &mut queue).await?;
                }
                Err(TryRecvError::Pending) => break,
                Err(TryRecvError::Closed) => panic!("TriggerUpdated channel was closed!"),
            }
        }
        trace!("no trigger updates pending - going around the scheduler loop again");

        // rather than update this every place we edit the queue just do it
        // once per loop - it's for monitoring purposes anyway
        server.queued_triggers.store(queue.len(), Ordering::SeqCst);
        statsd
            .gauge_with_tags("triggers.queued", queue.len() as u64)
            .send();

        if queue.is_empty() {
            debug!("no triggers queued, waiting for a trigger update");

            *server.waiting_for_trigger_id.lock().await = None;

            let trigger_update = trigger_rx
                .recv()
                .await
                .expect("TriggerUpdate channel was closed!");
            update_trigger(&server, trigger_update, &mut queue).await?;
            continue;
        }

        #[cfg(debug_assertions)]
        {
            let queue_copy = queue.clone();
            trace!(
                "dumping the first 10 (of total {}) triggers in the queue:",
                queue_copy.len()
            );
            for trigger in queue_copy.into_iter_sorted().take(10) {
                trace!(
                    "    {}: {} {}",
                    trigger.scheduled_datetime.to_rfc3339(),
                    trigger.trigger_datetime.to_rfc3339(),
                    trigger.trigger_id
                );
            }
        }

        let next_triggertime = queue.pop().expect("queue shouldn't be empty now");

        let delay = next_triggertime.scheduled_datetime - Utc::now();
        if delay > Duration::zero() {
            info!(trigger_id=?next_triggertime.trigger_id,
                "sleeping {} until next trigger", format_duration_approx(delay));

            *server.waiting_for_trigger_id.lock().await = Some(next_triggertime.trigger_id);

            tokio::select! {
                Some(trigger_update) = trigger_rx.recv() => {
                    trace!("received a trigger update while sleeping");

                    // put the trigger we slept on back in the queue
                    // update trigger might delete it, or we might select it as the next trigger
                    queue.push(next_triggertime);

                    update_trigger(&server, trigger_update, &mut queue).await?;
                }
                _ = time::sleep(delay.to_std()?) => {
                    trace!("sleep completed, no updates");
                    requeue_next_triggertime(&server, &next_triggertime, &mut queue).await?;
                    activate_trigger(&server, next_triggertime, TaskPriority::Normal).await?;
                }
            }
        } else {
            warn!("overslept trigger: {}", delay);
            requeue_next_triggertime(&server, &next_triggertime, &mut queue).await?;
            activate_trigger(&server, next_triggertime, TaskPriority::Normal).await?;
        }
    }
}

async fn activate_trigger(
    server: &Server,
    trigger_time: TriggerTime,
    priority: TaskPriority,
) -> Result<()> {
    let pool = server.db_pool.clone();

    let mut conn = pool.acquire().await?;
    let mut txn = conn.begin().await?;

    let tokens_to_tx = do_activate_trigger(&pool, &mut txn, trigger_time).await?;

    txn.commit().await?;
    trace!("done activating trigger: {}", trigger_time);

    // after committing the transaction we can tell the token processor to check thresholds
    send_to_token_processor(server, tokens_to_tx, priority).await?;

    Ok(())
}

#[derive(sqlx::FromRow)]
struct TriggerEdge {
    task_id: Uuid,
    edge_offset: Option<i64>,
}

async fn do_activate_trigger(
    pool: &PgPool,
    txn: &mut Transaction<'_, Postgres>,
    trigger_time: TriggerTime,
) -> Result<Vec<Token>> {
    debug!(trigger_id=?trigger_time.trigger_id,
        trigger_datetime=?trigger_time.trigger_datetime.to_rfc3339(),
        "activating trigger");

    let mut cursor = sqlx::query_as(
        "SELECT
            task_id,
            edge_offset
        FROM trigger_edge te
        WHERE trigger_id = $1",
    )
    .bind(trigger_time.trigger_id)
    .fetch(pool);

    let mut tokens_to_tx = Vec::new();

    while let Some(TriggerEdge {
        task_id,
        edge_offset,
    }) = cursor.try_next().await?
    {
        let token = Token {
            task_id,
            trigger_datetime: trigger_time.trigger_datetime
                + Duration::seconds(edge_offset.unwrap_or(0)),
        };

        increment_token(txn, &token).await?;
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
    .execute(txn.as_mut())
    .await?;

    Ok(tokens_to_tx)
}

async fn catchup_trigger(
    server: &Server,
    trigger: &Trigger,
    queue: &mut Queue,
) -> anyhow::Result<()> {
    debug!(trigger_id=?trigger.id, "checking trigger for any catchup");

    let pool = server.db_pool.clone();

    let mut tokens_to_tx = Vec::new();

    let mut conn = pool.acquire().await?;
    let mut txn = conn.begin().await?;

    let period = trigger.period()?;

    if trigger.catchup != Catchup::None {
        if let Some(earliest) = trigger.earliest_trigger_datetime {
            if trigger.start_datetime < earliest {
                // start date moved backwards
                debug!(trigger_id=?trigger.id,
                    "start date has moved backwards: {} -> {}",
                    earliest, trigger.start_datetime
                );

                let mut next = trigger.start_datetime;
                while next < earliest {
                    let mut tokens = do_activate_trigger(&pool, &mut txn, trigger.at(next)).await?;
                    tokens_to_tx.append(&mut tokens);
                    next = next + &period;
                }
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
        if trigger.catchup != Catchup::None {
            let mut tokens = do_activate_trigger(&pool, &mut txn, trigger.at(next)).await?;
            tokens_to_tx.append(&mut tokens);
        }
        next = next + &period;
    }

    if trigger.end_datetime.is_none() || next < trigger.end_datetime.unwrap() {
        // push one trigger in the future
        trace!(trigger_id=?trigger.id, "queueing trigger at {}", next);
        queue.push(trigger.at(next));
    }

    txn.commit().await?;

    match trigger.catchup {
        Catchup::None => assert_eq!(
            tokens_to_tx.len(),
            0,
            "Catchup::None should never have any tokens_to_tx"
        ),
        Catchup::Earliest => tokens_to_tx.sort_by_key(|token| token.trigger_datetime),
        Catchup::Latest => {
            tokens_to_tx.sort_by_key(|token| std::cmp::Reverse(token.trigger_datetime))
        }
        Catchup::Random => tokens_to_tx.shuffle(&mut thread_rng()),
    }

    send_to_token_processor(server, tokens_to_tx, TaskPriority::BackFill).await?;

    Ok(())
}

async fn send_to_token_processor(
    server: &Server,
    tokens_to_tx: Vec<Token>,
    priority: TaskPriority,
) -> Result<()> {
    let mut token_tx = server.post_office.post_mail::<ProcessToken>().await?;

    for token in tokens_to_tx {
        token_tx
            .send(ProcessToken::Increment(token, priority))
            .await?;
    }

    Ok(())
}

async fn update_trigger(
    server: &Server,
    trigger_update: TriggerChange,
    queue: &mut Queue,
) -> Result<()> {
    match trigger_update {
        TriggerChange::Add(uuids) => {
            for uuid in uuids {
                update_one_trigger(server, uuid, queue).await?;
            }
        }
        TriggerChange::Remove(uuids) => {
            for uuid in uuids {
                remove_trigger(uuid, queue);
            }
        }
    }

    Ok(())
}

fn remove_trigger(uuid: Uuid, queue: &mut Queue) {
    // de-heapify the triggers and delete the one we are updating
    let mut triggers = queue
        .drain()
        .filter(|t| t.trigger_id != uuid)
        .collect::<Vec<_>>();

    // now heapify them again
    queue.extend(triggers.drain(..));
}

// TODO - we receive the updates in a batch now so make use of that to avoid
// multiple heap rebuilds and queries
async fn update_one_trigger(server: &Server, uuid: Uuid, queue: &mut Queue) -> Result<()> {
    let pool = server.db_pool.clone();

    debug!(trigger_id=?uuid, "updating trigger");

    remove_trigger(uuid, queue);

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
            trigger_offset,
            catchup
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
        catchup_trigger(server, &trigger, queue).await?;
    } else {
        debug!(trigger_id=?uuid,
            "trigger has been paused, it has been removed from the queue"
        );
    }

    Ok(())
}

async fn requeue_next_triggertime(
    server: &Server,
    next_triggertime: &TriggerTime,
    queue: &mut Queue,
) -> Result<()> {
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
            trigger_offset,
            catchup
        FROM trigger
        WHERE id = $1
    ",
    )
    .bind(next_triggertime.trigger_id)
    .fetch_one(&server.db_pool)
    .await?;

    let next_datetime = next_triggertime.trigger_datetime + &trigger.period()?;

    if trigger.end_datetime.is_none() || next_datetime < trigger.end_datetime.unwrap() {
        let requeue = trigger.at(next_datetime);

        trace!(trigger_id=?requeue.trigger_id,
            "queueing next time: {}", requeue.trigger_datetime.to_rfc3339());

        queue.push(requeue);
    }

    Ok(())
}

pub async fn trigger_cluster_changes(server: Arc<Server>) -> Result<!> {
    let mut cluster_rx = server.on_cluster_membership_change.subscribe();
    let mut change_tx = server.post_office.post_mail::<TriggerChange>().await?;
    let mut current_triggers = HashSet::new();

    loop {
        info!("cluster membership changed");
        let triggers = get_all_triggers(&server.db_pool).await?;

        let new_triggers = {
            let rendezvous = cluster_rx.borrow();

            let mut new_triggers = HashSet::new();
            for trigger in triggers {
                if rendezvous.item_is_mine(&server.node_id, &trigger) {
                    new_triggers.insert(trigger);
                }
            }
            new_triggers
        };

        let to_remove: Vec<_> = current_triggers
            .difference(&new_triggers)
            .map(deref)
            .collect();
        trace!("removing triggers: {:?}", to_remove);
        info!("removing {} triggers", to_remove.len());
        change_tx.send(TriggerChange::Remove(to_remove)).await?;

        let to_add: Vec<_> = new_triggers
            .difference(&current_triggers)
            .map(deref)
            .collect();
        trace!("adding triggers: {:?}", to_add);
        info!("adding {} triggers", to_add.len());
        change_tx.send(TriggerChange::Add(to_add)).await?;

        current_triggers = new_triggers;

        cluster_rx.changed().await?;
    }
}

pub async fn trigger_update(server: Arc<Server>, update: TriggerUpdate) -> Result<()> {
    let mut change_tx = server.post_office.post_mail::<TriggerChange>().await?;

    let TriggerUpdate(uuids) = update;
    trace!(?uuids, "got trigger update");

    let to_add: Vec<_> = {
        let rendezvous = server.on_cluster_membership_change.borrow();
        uuids
            .iter()
            .filter(|trigger| rendezvous.item_is_mine(&server.node_id, trigger))
            .map(deref)
            .collect()
    };

    trace!(?to_add, "filtered triggers by rendezvous hash");

    if !to_add.is_empty() {
        change_tx.send(TriggerChange::Add(to_add)).await?;
    }

    Ok(())
}

async fn get_all_triggers(db: &PgPool) -> Result<HashSet<Uuid>> {
    let triggers = sqlx::query_as(
        "
        SELECT t.id
        FROM trigger t
        JOIN job j ON t.job_id = j.id
        WHERE NOT j.paused",
    )
    .fetch_all(db)
    .await?;

    Ok(triggers.into_iter().map(first).collect())
}
