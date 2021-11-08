import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader, Button, notification, Popconfirm,
        Row, Col, Drawer, Descriptions, Skeleton } from 'antd';

import { ExclamationCircleOutlined, EllipsisOutlined } from '@ant-design/icons';

import axios from 'axios';

import Body from '../components/Body.jsx';
import State from '../components/State.jsx';
import Graph from '../components/Graph.jsx';
import ActivateToken from '../components/ActivateToken.jsx';

const { Content } = Layout;

function Json({children}) {
    if (typeof(children) == 'string') {
        return <pre>{children}</pre>;
    } else {
        return <pre>{JSON.stringify(children)}</pre>;
    }
}

class TokenRuns extends Component {
    constructor(props) {
        super(props);

        this.columns = this.makeColumns();

        this.state = {
            runs: [],
            task: null,
        }
    }

    makeColumns() {
        return [
          {
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

    async fetchRuns(task_id, trigger_datetime) {
        try {
            let resp1 = await axios.get(`/api/tasks/${task_id}/runs/${trigger_datetime}`);
            let runs = resp1.data;

            let resp2 = await axios.get(`/api/tasks/${task_id}`);
            this.setState({
                task: resp2.data,
                runs: runs,
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

    componentDidUpdate(prevProps) {
        if (this.props.task_id !== prevProps.task_id) {
            this.componentDidMount()
        }
    }

    render() {
        const { task_id, trigger_datetime, visible, onClose } = this.props;
        const { runs, task } = this.state;

        return (
            <Drawer title={`Task Runs for {task.task_name}`}
                    placement="bottom"
                    size="large"
                    height={736} // todo - remove after upgrading
                    onClose={onClose}
                    visible={visible}>

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
                        <Json>{task?.image}</Json>
                    </Descriptions.Item>
                    <Descriptions.Item label="Args">
                        <Json>{task?.args}</Json>
                    </Descriptions.Item>
                    <Descriptions.Item label="Env">
                        <Json>{task?.env}</Json>
                    </Descriptions.Item>
                </Descriptions>

                <ActivateToken
                    type="primary" size="default"
                    task_id={task_id}
                    trigger_datetime={trigger_datetime} />

                <Table columns={this.columns}
                    dataSource={runs}
                    pagination={{position: ['bottomLeft']}}
                    />
            </Drawer>
        );
    }
}

export default TokenRuns;
