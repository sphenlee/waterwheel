import React, { Component, Fragment, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Descriptions } from 'antd';
import { ColumnsType } from "antd/es/table";
import { RightOutlined, DownOutlined } from "@ant-design/icons";

import axios from 'axios';
import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';
dayjs.extend(relativeTime);

import State from '../components/State';
import Priority from '../components/Priority';
import ActivateToken from '../components/ActivateToken';
import { datetime, interval } from "../types/common";
import { Task, TaskRun } from "../types/Task";
import RelDate from '../components/Date';

const { Content } = Layout;

function Json({json}: {json: any}) {
    if (typeof(json) == 'string') {
        return <pre>{json}</pre>;
    } else {
        return <pre>{JSON.stringify(json)}</pre>;
    }
}

function expandedRowRender(record: TaskRun) {
    return (
        <Descriptions
                size="small"
                bordered
                labelStyle={{
                    fontWeight: "bold"
                }}
                contentStyle={{
                    background: "#fff"
                }}>
            <Descriptions.Item label="Task Run Id">
                {record.task_run_id}
            </Descriptions.Item>
            <Descriptions.Item label="Queued Time">
                <RelDate>{record.queued_datetime ?? ''}</RelDate>
            </Descriptions.Item>
            <Descriptions.Item label="Start Time">
                <RelDate>{record.started_datetime}</RelDate>
            </Descriptions.Item>
            <Descriptions.Item label="Start Delay">
                {dayjs(record.started_datetime).to(record.finish_datetime)}
            </Descriptions.Item>
            <Descriptions.Item label="Finished Time">
                {record.finish_datetime &&
                    <RelDate>{record.finish_datetime}</RelDate>
                }
            </Descriptions.Item>
            <Descriptions.Item label="Running Duration">
                {record.finish_datetime &&
                    dayjs(record.started_datetime).to(record.finish_datetime)
                }
            </Descriptions.Item>
            <Descriptions.Item label="Worker">
                <Link to={`/workers/${record.worker_id}`}>
                    {record.worker_id}
                </Link>
            </Descriptions.Item>
        </Descriptions>
    );
}

function makeColumns(): ColumnsType<TaskRun> {
    return [
        /*{
        title: 'Id',
        dataIndex: 'task_run_id',
        key: 'task_run_id',
        },*/{
        title: 'Attempt',
        dataIndex: 'attempt',
        key: 'attempt',
        },{
        title: 'State',
        dataIndex: 'state',
        render: text => <State state={text} />,
        },{
        title: 'Priority',
        dataIndex: 'priority',
        render: text => <Priority priority={text} />,
        },{
        title: 'Logs',
        dataIndex: 'task_run_id',
        render: text => <Link to={`/logs/${text}`}>logs</Link>,
        }
    ];
}

const columns = makeColumns();

function TokenRuns({task_id, trigger_datetime}) {
    const [runs, setRuns] = useState([] as TaskRun[]);
    const [task, setTask] = useState<Task>();
    
    async function fetchRuns() {
        try {
            const [runs, task] = await Promise.all([
                axios.get<TaskRun[]>(`/api/tasks/${task_id}/runs/${trigger_datetime}`),
                axios.get<Task>(`/api/tasks/${task_id}`)
            ]);

            setRuns(runs.data);            
            setTask(task.data);
        } catch(e) {
            console.log(e);
        }
    }

    useEffect(() => {
        fetchRuns();
        const interval = setInterval(() => fetchRuns(), 2000);
    
        return () => clearInterval(interval);
    });

    
    if (!task) {
        return '';
    }

    return (
        <Fragment>
            <h2>{`${task.task_name} @ ${trigger_datetime}`}</h2>

            <ActivateToken
                type="primary"
                size="middle"
                task_id={task_id ?? ''}
                trigger_datetime={trigger_datetime ?? ''} />


            <Descriptions
                    size="small"
                    bordered
                    labelStyle={{
                        fontWeight: "bold"
                    }}
                    contentStyle={{
                        background: "#fff"
                    }}>
                <Descriptions.Item label="Image" span={3}>
                    <pre>{task?.image}</pre>
                </Descriptions.Item>
                <Descriptions.Item label="Args" span={3}>
                    <Json json={task?.args} />
                </Descriptions.Item>
                <Descriptions.Item label="Env" span={3}>
                    <Json json={task?.env ?? {}} />
                </Descriptions.Item>
            </Descriptions>

            <Table columns={columns}
                rowKey="task_run_id"
                dataSource={runs}
                pagination={{position: ['bottomLeft']}}
                expandable={{
                    expandedRowRender: record => expandedRowRender(record),
                    expandRowByClick: true,
                    expandIcon: ({ expanded, onExpand, record }) =>
                        expanded ? (
                            <DownOutlined onClick={e => onExpand(record, e)} />
                        ) : (
                            <RightOutlined onClick={e => onExpand(record, e)} />
                        )
                }}
                />
        </Fragment>
    );
}

export default TokenRuns;
