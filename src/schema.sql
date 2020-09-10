CREATE TABLE IF NOT EXISTS project (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    description VARCHAR
);

CREATE TABLE IF NOT EXISTS job (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    project_id UUID NOT NULL REFERENCES project(id),
    description VARCHAR,
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

CREATE TABLE IF NOT EXISTS trigger_edge (
    trigger_id UUID NOT NULL REFERENCES trigger(id),
    task_id UUID NOT NULL REFERENCES task(id),
    UNIQUE(trigger_id, task_id)
);

CREATE TABLE IF NOT EXISTS task_edge (
    parent_task_id UUID NOT NULL REFERENCES task(id),
    child_task_id UUID NOT NULL REFERENCES task(id),
    kind VARCHAR NOT NULL,
    UNIQUE(parent_task_id, child_task_id, kind)
);
