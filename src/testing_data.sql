INSERT INTO trigger(
	id,
	name,
	job_id,
	start_datetime,
	end_datetime,
	earliest_trigger_datetime,
	latest_trigger_datetime,
	period
)
VALUES
( '09e9c36c-5f97-46bc-912b-dce8a3a7cb4d',
    'trigger_a',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    '2020-01-01T00:00:00Z',
    '2021-01-01T00:00:00Z',
    '2020-01-01T00:00:00Z',
     current_timestamp,
    60
),
( '328b5b11-4cc3-4321-95d8-7b1d53064390',
    'trigger_b',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    '2020-01-01T00:00:30Z',
    '2021-01-01T00:00:30Z',
    '2020-01-01T00:00:30Z',
     current_timestamp,
    60
);

INSERT INTO task(
    id,
    name,
    job_id,
    threshold,
    image,
    args,
    env
)
VALUES
(   '6fd8b847-9fe7-4944-b0f9-8db428d949b5',
    'step_1b',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    1,
    'bash',
    ARRAY['echo', 'step_1b'],
    ARRAY[]::VARCHAR[]
),
(   '7daf6f96-c389-4c38-976e-34e8afc04765',
    'step_1a',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    1,
    'bash',
    ARRAY['echo', 'step_1a'],
    ARRAY[]::VARCHAR[]
),
(   'e30a8c0e-f891-47e8-9668-88c4a692b781',
    'step_2',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    2,
    'bash',
    ARRAY['echo', 'step_2'],
    ARRAY[]::VARCHAR[]
),
(   'cb6c0a85-7687-467b-b443-df2db30042d1',
    'step_0',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    1,
    'bash',
    ARRAY['echo', 'step_0'],
    ARRAY[]::VARCHAR[]
),
 (   '1daf309d-9bd7-4d07-b564-1595d6ef436d',
     'task_b',
     '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
     1,
     'bash',
     ARRAY['echo', 'task_b'],
     ARRAY[]::VARCHAR[]
 );

INSERT INTO trigger_edge(trigger_id, task_id)
VALUES
('328b5b11-4cc3-4321-95d8-7b1d53064390', '1daf309d-9bd7-4d07-b564-1595d6ef436d'),
('09e9c36c-5f97-46bc-912b-dce8a3a7cb4d', 'cb6c0a85-7687-467b-b443-df2db30042d1');

INSERT INTO task_edge(parent_task_id, child_task_id)
VALUES
('6fd8b847-9fe7-4944-b0f9-8db428d949b5', 'e30a8c0e-f891-47e8-9668-88c4a692b781'),
('7daf6f96-c389-4c38-976e-34e8afc04765', 'e30a8c0e-f891-47e8-9668-88c4a692b781'),
('cb6c0a85-7687-467b-b443-df2db30042d1', '7daf6f96-c389-4c38-976e-34e8afc04765'),
('cb6c0a85-7687-467b-b443-df2db30042d1', '6fd8b847-9fe7-4944-b0f9-8db428d949b5');

