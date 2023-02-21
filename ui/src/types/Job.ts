import { datetime, uuid } from "./common";

export type Job = {
    id: uuid;
    project_id: uuid;
    name: string;
    description: string;
    paused: boolean;
};

export type JobExtra = Job & {
    project: string;
    raw_definition: string;
    active_tasks: number;
    waiting_tasks: number;
    failed_tasks_last_hour: number;
    succeeded_tasks_last_hour: number;
    error_tasks_last_hour: number;
};

export type JobQuery = Partial<{
    project: string;
    name: string;
}>;

export type JobTrigger = {
    trigger_id: uuid;
    trigger_name: string;
    start_datetime: datetime;
    end_datetime: datetime;
    earliest_trigger_datetime: datetime | null;
    latest_trigger_datetime: datetime | null;
    period: number | null;
    cron: string | null;
    trigger_offset: string | null;
    catchup: string | null;
};

export type Trigger = {
    trigger_id: uuid;
    trigger_name: string;
    job_id: uuid;
    job_name: string;
    project_id: uuid;
    project_name: string;

    times: TriggerTime[];
};

export type TriggerTime = {
    trigger_datetime: datetime;
    success: number;
    running: number;
    failure: number;
    waiting: number;
};

export type JobGraph = {
    nodes: JobGraphNode[];
    edges: JobGraphEdge[];
};

export type JobGraphNode = {
    id: uuid;
    kind: string;
    name: string;
    job_id: uuid;
    state: States | null;
};

export type JobGraphEdge = {
    from: uuid;
    to: uuid;
    kind: string;
};

export type States = 'active'
    | 'success'
    | 'failure'
    | 'waiting'
    | 'running'
    | 'timeout'
    | 'error'
    | 'retry'
    | 'cancelled';
