import { datetime } from "./common";

export type TaskDuration = {
    trigger_datetime: datetime;
    duration: number | null;
    task_name: string;
};

export type TaskDurationList = {
    duration: TaskDuration[];
};

export type GetTaskDurationQuery = {
    before: datetime | null;
    limit: number | null;
};
