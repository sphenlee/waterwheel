import { datetime, uuid } from "./common"

export type WorkerState = {
    uuid: uuid;
    addr: string;
    version: string;
    last_seen_datetime: datetime;
    running_tasks: number;
    total_tasks: number;
    status: string;
};

export type Worker = {
    last_seen_datetime: datetime;
    running_tasks: number;
    total_tasks: number;
    status: string;
    version: string;
    tasks: WorkerTask[];
};

export type WorkerTask = {
    job_id: uuid;
    job_name: string;
    project_id: string;
    project_name: string;
    task_run_id: uuid;
    task_name: string;
    trigger_datetime: datetime;
    queued_datetime: datetime;
    started_datetime: datetime;
    finish_datetime: datetime;
    state: string;
};
