import React, { Component } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader, Button, notification, Popconfirm,
        Row, Col, Drawer, Descriptions, Skeleton, Space } from 'antd';

import { ExclamationCircleOutlined, EllipsisOutlined } from '@ant-design/icons';
import { RightOutlined, DownOutlined } from "@ant-design/icons";

import axios from 'axios';
import Moment from 'react-moment';

import State from '../components/State';
import Priority from '../components/Priority';
import ActivateToken from '../components/ActivateToken';
import { ColumnsType } from "antd/lib/table";
import { datetime } from "../types/common";
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

type TokenRunsProps = {
    task_id: string | null;
    trigger_datetime: datetime | null;
};
type TokenRunsState = {
    runs: TaskRun[];
    task?: Task;
};

function expandedRowRender(record: TaskRun) {
    return (
        <Descriptions
                size="small"
                bordered
                column={2}
                labelStyle={{
                    fontWeight: "bold"
                }}
                contentStyle={{
                    background: "#fff"
                }}>
            <Descriptions.Item label="Run Id" span={2}>
                {record.task_run_id}
            </Descriptions.Item>
            <Descriptions.Item label="Queued Time" span={2}>
                <RelDate>{record.queued_datetime ?? ''}</RelDate>
            </Descriptions.Item>
            <Descriptions.Item label="Start Time">
                <RelDate>{record.started_datetime}</RelDate>
            </Descriptions.Item>
            <Descriptions.Item label="Start Delay">
                <Moment duration={record.queued_datetime} date={record.started_datetime} />
            </Descriptions.Item>
            <Descriptions.Item label="Finished Time">
                {record.finish_datetime &&
                    <RelDate>{record.finish_datetime}</RelDate>
                }
            </Descriptions.Item>
            <Descriptions.Item label="Running Duration">
                {record.finish_datetime &&
                    <Moment duration={record.started_datetime} date={record.finish_datetime} />
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


class TokenRuns extends Component<TokenRunsProps, TokenRunsState> {
    columns: ColumnsType<TaskRun>;
    interval: NodeJS.Timeout;

    constructor(props: TokenRunsProps) {
        super(props);

        this.columns = this.makeColumns();

        this.state = {
            runs: [],
        }
    }

    makeColumns(): ColumnsType<TaskRun> {
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
          }
        ];
    }

    async fetchRuns(task_id: string, trigger_datetime: string) {
        try {
            let resp1 = await axios.get<TaskRun[]>(`/api/tasks/${task_id}/runs/${trigger_datetime}`);

            let resp2 = await axios.get<Task>(`/api/tasks/${task_id}`);
            this.setState({
                runs: resp1.data,
                task: resp2.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        const {task_id, trigger_datetime} = this.props;

        if (task_id !== null && trigger_datetime !== null) {
            this.interval = setInterval(() => this.fetchRuns(task_id, trigger_datetime), 2000);
            this.fetchRuns(task_id, trigger_datetime);
        }
    }

    componentDidUpdate(prevProps: TokenRunsProps) {
        if (this.props.task_id !== prevProps.task_id
            || this.props.trigger_datetime !== prevProps.trigger_datetime)
        {
            this.componentWillUnmount();
            this.componentDidMount();
        }
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { task_id, trigger_datetime, } = this.props;
        const { runs, task } = this.state;

        if (!task) {
            return '';
        }

        return (
            <Space direction="vertical">
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

                <Table columns={this.columns}
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
            </Space>
        );
    }
}

export default TokenRuns;
