import { datetime, uuid } from "./common"

export type SchedulerState = {
    uuid: uuid;
    version: string;
    last_seen_datetime: datetime;
    queued_triggers: number;
    waiting_for_trigger_id: uuid | null;
    waiting_for_trigger_job_id: uuid | null;
    status: string;
};

