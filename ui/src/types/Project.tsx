import { uuid } from "./common";

export type Project = {
    id: uuid;
    name: string;
    description: string;
};

export type ProjectExtra = Project & {
    num_jobs: number;
    running_tasks: number;
    waiting_tasks: number;
    failed_tasks_last_hour: number;
    succeeded_tasks_last_hour: number;
    error_tasks_last_hour: number;
};

export type ProjectJob = {
    job_id: uuid;
    name: string;
    description: string;
    paused: boolean;
    success: number;
    running: number;
    failure: number;
    waiting: number;
    error: number;
};
