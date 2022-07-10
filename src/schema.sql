CREATE TABLE IF NOT EXISTS project (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    description VARCHAR,
    config JSONB
);

CREATE TABLE IF NOT EXISTS job (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    project_id UUID NOT NULL REFERENCES project(id),
    description VARCHAR,
    paused BOOLEAN,
    raw_definition VARCHAR,
    UNIQUE(project_id, name) INCLUDE (id)
);

CREATE TABLE IF NOT EXISTS trigger (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    job_id UUID NOT NULL REFERENCES job(id),
    start_datetime TIMESTAMP WITH TIME ZONE NOT NULL,
    end_datetime TIMESTAMP WITH TIME ZONE,
    earliest_trigger_datetime TIMESTAMP WITH TIME ZONE,
    latest_trigger_datetime TIMESTAMP WITH TIME ZONE,
    period BIGINT,
    cron VARCHAR,
    trigger_offset BIGINT,
    catchup VARCHAR NOT NULL,
    UNIQUE(job_id, name) INCLUDE (id)
);

CREATE TABLE IF NOT EXISTS task (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    job_id UUID NOT NULL REFERENCES job(id),
    threshold INT,
    image VARCHAR,
    args VARCHAR[],
    env VARCHAR[],
    UNIQUE(job_id, name) INCLUDE (id)
);

CREATE TABLE IF NOT EXISTS token (
    task_id UUID NOT NULL REFERENCES task(id),
    trigger_datetime TIMESTAMP WITH TIME ZONE NOT NULL,
    count INT,
    state VARCHAR,
    UNIQUE(task_id, trigger_datetime)
);

CREATE TABLE IF NOT EXISTS worker (
    id UUID PRIMARY KEY,
    addr VARCHAR,
    last_seen_datetime TIMESTAMP WITH TIME ZONE NOT NULL,
    running_tasks INT,
    total_tasks INT,
    version VARCHAR
);

CREATE TABLE IF NOT EXISTS task_run (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES task(id),
    trigger_datetime TIMESTAMP WITH TIME ZONE NOT NULL,
    queued_datetime TIMESTAMP WITH TIME ZONE NOT NULL,
    started_datetime TIMESTAMP WITH TIME ZONE,
    finish_datetime TIMESTAMP WITH TIME ZONE,
    worker_id UUID REFERENCES worker(id),
    state VARCHAR,
    priority VARCHAR NOT NULL
);

-- this may not be needed long term, but useful for tracking down a weird bug
CREATE TABLE IF NOT EXISTS task_run_history (
    task_run_id UUID NOT NULL REFERENCES task_run(id),
    change_datetime TIMESTAMP WITH TIME ZONE NOT NULL,
    worker_id UUID REFERENCES worker(id),
    scheduler_id UUID NOT NULL,
    state VARCHAR
);

CREATE INDEX IF NOT EXISTS task_run_by_state
    ON task_run(state, finish_datetime, task_id);

CREATE TABLE IF NOT EXISTS trigger_edge (
    trigger_id UUID NOT NULL REFERENCES trigger(id),
    task_id UUID NOT NULL REFERENCES task(id),
    edge_offset BIGINT,
    UNIQUE(trigger_id, task_id)
);

CREATE TABLE IF NOT EXISTS task_edge (
    parent_task_id UUID NOT NULL REFERENCES task(id),
    child_task_id UUID NOT NULL REFERENCES task(id),
    kind VARCHAR NOT NULL,
    edge_offset BIGINT,
    UNIQUE(parent_task_id, child_task_id, kind)
);

CREATE TABLE IF NOT EXISTS global_stash (
    name VARCHAR PRIMARY KEY,
    data BYTEA
);

CREATE TABLE IF NOT EXISTS project_stash (
    project_id UUID NOT NULL REFERENCES project(id),
    name VARCHAR NOT NULL,
    data BYTEA,
    UNIQUE(project_id, name)
);

CREATE TABLE IF NOT EXISTS job_stash (
    job_id UUID NOT NULL REFERENCES job(id),
    trigger_datetime TIMESTAMP WITH TIME ZONE NOT NULL,
    name VARCHAR NOT NULL,
    data BYTEA,
    UNIQUE(job_id, trigger_datetime, name)
);
