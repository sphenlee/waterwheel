INSERT INTO trigger(
	id,
	name,
	job_id,
	start_datetime,
	end_datetime,
	earliest_trigger_datetime,
	latest_trigger_datetime, period
)
VALUES
( 'd585cf60-4937-4a53-9cb1-20b111356c4d',
  'daily',
  '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
  '2020-01-01T00:00:00Z',
  '2020-02-01T00:00:00Z',
  '2020-01-05T00:00:00Z',
  '2020-01-29T00:00:00Z',
  86400
  ),
( 'b5158fbe-cbc3-4910-99e5-e83c554551a0',
  'daily2',
  '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
  '2020-01-01T00:00:00Z',
  '2020-02-01T00:00:00Z',
  '2020-01-05T00:00:00Z',
  '2020-01-29T00:00:00Z',
  86400
  ),
( '09e9c36c-5f97-46bc-912b-dce8a3a7cb4d',
    'future',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    '2020-01-01T00:00:00Z',
    '2021-01-01T00:00:00Z',
    '2020-01-01T00:00:00Z',
    '2020-08-17T00:00:00Z',
    60
),
( '328b5b11-4cc3-4321-95d8-7b1d53064390',
    'half_minute',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    '2020-01-01T00:00:30Z',
    '2021-01-01T00:00:30Z',
    '2020-01-01T00:00:30Z',
    '2020-08-17T10:50:30Z',
    60
);

INSERT INTO task(id, name, job_id, threshold)
VALUES
(   '6fd8b847-9fe7-4944-b0f9-8db428d949b5',
    'step0',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    1
),
(   '7daf6f96-c389-4c38-976e-34e8afc04765',
    'step1',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    1
),
(   'e30a8c0e-f891-47e8-9668-88c4a692b781',
    'step2',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    2
),
(   'cb6c0a85-7687-467b-b443-df2db30042d1',
    'every_minute',
    '803b4cc2-f4d6-45d2-93bb-f92fbb2eb9b9',
    1
);

INSERT INTO trigger_edge(trigger_id, task_id)
VALUES
('d585cf60-4937-4a53-9cb1-20b111356c4d', '6fd8b847-9fe7-4944-b0f9-8db428d949b5'),
('b5158fbe-cbc3-4910-99e5-e83c554551a0', 'e30a8c0e-f891-47e8-9668-88c4a692b781'),
('09e9c36c-5f97-46bc-912b-dce8a3a7cb4d', 'cb6c0a85-7687-467b-b443-df2db30042d1');

INSERT INTO task_edge(parent_task_id, child_task_id)
VALUES
('6fd8b847-9fe7-4944-b0f9-8db428d949b5', '7daf6f96-c389-4c38-976e-34e8afc04765'),
('7daf6f96-c389-4c38-976e-34e8afc04765', 'e30a8c0e-f891-47e8-9668-88c4a692b781');

