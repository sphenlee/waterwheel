CREATE TABLE IF NOT EXISTS project (
    id UUID PRIMARY KEY,
    name VARCHAR UNIQUE
);

CREATE TABLE IF NOT EXISTS job (
    id UUID PRIMARY KEY,
    name VARCHAR,
    project_id UUID,
    raw_definition VARCHAR,
    UNIQUE(project_id, name) INCLUDE (id)
);

CREATE TABLE IF NOT EXISTS trigger (
    id UUID PRIMARY KEY,
    name VARCHAR,
    job_id UUID,
    start_datetime TIMESTAMP WITH TIME ZONE,
    end_datetime TIMESTAMP WITH TIME ZONE,
    earliest_trigger_datetime TIMESTAMP WITH TIME ZONE,
    latest_trigger_datetime TIMESTAMP WITH TIME ZONE,
    period BIGINT,
    UNIQUE(job_id, name) INCLUDE (id)
);

CREATE TABLE IF NOT EXISTS task (
    id UUID PRIMARY KEY,
    name VARCHAR,
    job_id VARCHAR,
    threshold INT,
    image VARCHAR,
    args VARCHAR[],
    env VARCHAR[],
    UNIQUE(job_id, name) INCLUDE (id)
);

CREATE TABLE IF NOT EXISTS token (
    task_id UUID,
    trigger_datetime TIMESTAMP WITH TIME ZONE,
    count INT,
    state VARCHAR,
    UNIQUE(task_id, trigger_datetime)
);

CREATE TABLE IF NOT EXISTS trigger_edge (
    trigger_id UUID,
    task_id UUID,
    UNIQUE(trigger_id, task_id)
);

CREATE TABLE IF NOT EXISTS task_edge (
    parent_task_id UUID,
    child_task_id UUID,
    UNIQUE(parent_task_id, child_task_id)
);
