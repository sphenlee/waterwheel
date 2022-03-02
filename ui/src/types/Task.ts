import { datetime, uuid } from "./common";

export type Task = {
    task_id: uuid;
    task_name: string;
    job_id: uuid;
    job_name: string;
    project_id: uuid;
    project_name: string;
    image: string | null;
    args: string[];
    env: string[] | null;
};

export type TaskDuration = {
    trigger_datetime: datetime;
    duration: number | null;
    task_name: string;
};

export type TaskDurationList = {
    duration: TaskDuration[];
};

export type TaskRun = {
    task_id: uuid;
    task_run_id: uuid;
    name: string;
    trigger_datetime: datetime;
    attempt: number;
    queued_datetime: datetime | null;
    started_datetime: datetime | null;
    finish_datetime: datetime | null;
    state: string;
    worker_id: uuid | null;
};

export type GetTaskDurationQuery = {
    before: datetime | null;
    limit: number | null;
};
