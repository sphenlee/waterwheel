type uuid = string;

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
