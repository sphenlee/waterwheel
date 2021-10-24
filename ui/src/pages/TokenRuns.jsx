import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader, Button, notification, Popconfirm,
        Row, Col, Drawer } from 'antd';

import { ExclamationCircleOutlined, EllipsisOutlined } from '@ant-design/icons';

import axios from 'axios';

import Body from '../components/Body.jsx';
import State from '../components/State.jsx';
import Graph from '../components/Graph.jsx';
import ActivateToken from '../components/ActivateToken.jsx';

const { Content } = Layout;

class TokenRuns extends Component {
    constructor(props) {
        super(props);

        this.columns = this.makeColumns();

        this.state = {
            runs: []
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
          }
        ];
    }

    async fetchRuns(task_id, trigger_datetime) {
        try {
            let resp = await axios.get(`/api/tasks/${task_id}/runs/${trigger_datetime}`);
            this.setState({
                runs: resp.data,
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
        const { runs } = this.state;

        return (
            <Drawer title="Task Runs"
                    placement="bottom"
                    size="large"
                    height={736} // todo - remove after upgrading
                    onClose={onClose}
                    visible={visible}>
                <Table columns={this.columns}
                    dataSource={runs}
                    pagination={{position: ['bottomLeft']}}
                    />
            </Drawer>
        );
    }
}

export default TokenRuns;
