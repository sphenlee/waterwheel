import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader, Button, notification, Popconfirm,
        Row, Col, Drawer, Descriptions, Skeleton } from 'antd';

import { ExclamationCircleOutlined, EllipsisOutlined } from '@ant-design/icons';

import axios from 'axios';

import State from '../components/State';
import ActivateToken from '../components/ActivateToken';
import { ColumnsType } from "antd/lib/table";
import { datetime } from "../types/common";
import { Task, TaskRun } from "../types/Task";

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
    trigger_datetime: datetime;
    visible: boolean;
    onClose: React.EventHandler<React.MouseEvent | React.KeyboardEvent>;
};
type TokenRunsState = {
    runs: TaskRun[];
    task?: Task;
};

class TokenRuns extends Component<TokenRunsProps, TokenRunsState> {
    columns: ColumnsType<TaskRun>;

    constructor(props: TokenRunsProps) {
        super(props);

        this.columns = this.makeColumns();

        this.state = {
            runs: [],
        }
    }

    makeColumns(): ColumnsType<TaskRun> {
        return [
          {
            title: 'Id',
            dataIndex: 'task_run_id',
            key: 'task_run_id',
          },{
            title: 'Attempt',
            dataIndex: 'attempt',
            key: 'attempt',
          },{
            title: 'State',
            dataIndex: 'state',
            render: text => <State state={text} />,
          },{
            title: 'Queued',
            dataIndex: 'queued_datetime',
          },{
            title: 'Started',
            dataIndex: 'started_datetime',
          },{
            title: 'Finished',
            dataIndex: 'finish_datetime',
          },{
            title: 'Worker Id',
            dataIndex: 'worker_id',
            render: text => (
                <Link to={`/workers/${text}`}>
                    {text}
                </Link>
              )
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

        if (task_id !== null) {
            this.fetchRuns(task_id, trigger_datetime);
        }
    }

    componentDidUpdate(prevProps: TokenRunsProps) {
        if (this.props.task_id !== prevProps.task_id) {
            this.componentDidMount()
        }
    }

    render() {
        const { task_id, trigger_datetime, visible, onClose } = this.props;
        const { runs, task } = this.state;

        return (
            <Drawer title={`Task Runs for ${task?.task_name ?? '...'}`}
                    placement="bottom"
                    // size="large"
                    height={736} // todo - remove after upgrading
                    onClose={onClose}
                    visible={visible}>

                <ActivateToken
                    type="primary"
                    size="middle"
                    task_id={task_id ?? ''}
                    trigger_datetime={trigger_datetime} />


                <Descriptions
                        size="small"
                        bordered
                        labelStyle={{
                            fontWeight: "bold"
                        }}
                        contentStyle={{
                            background: "#fff"
                        }}>
                    <Descriptions.Item label="Image">
                        <Json json={task?.image} />
                    </Descriptions.Item>
                    <Descriptions.Item label="Args">
                        <Json json={task?.args} />
                    </Descriptions.Item>
                    <Descriptions.Item label="Env">
                        <Json json={task?.env} />
                    </Descriptions.Item>
                </Descriptions>


                <Table columns={this.columns}
                    dataSource={runs}
                    pagination={{position: ['bottomLeft']}}
                    />
            </Drawer>
        );
    }
}

export default TokenRuns;
