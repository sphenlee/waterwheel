import { datetime, uuid } from "./common";

export type Token = {
    task_id: uuid;
    task_name: string;
    trigger_datetime: datetime;
    state: string;
};

export type TokenOverview = {
    tokens: TokensRow[],
    tasks: string[],
};

export type TokensRow = {
    trigger_datetime: datetime;
    task_states: Record<string, TokenState>;
};

export type TokenState = {
    task_name: string;
    task_id: uuid;
    state: string;
};
