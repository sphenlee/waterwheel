import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Select, notification, Popconfirm } from 'antd';
import { geekblue, lime, red, grey, yellow, orange } from '@ant-design/colors';
import axios from 'axios';
import styled from 'styled-components';

import {
  CheckCircleOutlined,
  SyncOutlined,
  CloseSquareOutlined,
  ExclamationCircleOutlined,
  ClockCircleOutlined,
  MinusCircleOutlined,
  WarningOutlined,
  QuestionCircleOutlined,
} from '@ant-design/icons';


import State from '../../components/State.jsx';

const { Option } = Select;

const HeaderCell = styled.td`
    writing-mode: vertical-rl;
`;

const Cell = styled.td`
    border-bottom: 1px solid #ddd;
    padding-right: 15px;
`;

const Row = styled.tr`
    padding-bottom: 15px;
`;


function iconForState(task) {
    if (task === undefined) {
        return '';
    }

    let state = task.state;

    if (state == 'active') {
        return <SyncOutlined style={{color: grey[5]}}/>;
    } else if (state == 'running') {
        return <SyncOutlined spin style={{color: geekblue[5]}}/>;
    } else if (state == 'waiting') {
        return <ClockCircleOutlined style={{color: grey[5]}}/>;
    } else if (state == 'success') {
        return <CheckCircleOutlined style={{color: lime[5]}}/>;
    } else if (state == 'failure') {
        return <CloseSquareOutlined style={{color: red[5]}}/>;
    } else if (state == 'error') {
        return <WarningOutlined style={{color: orange[5]}}/>;
    } else {
        return '';
    }
}

async function activateToken(trigger_datetime, task_id) {
    await axios.put(`/api/tasks/${task_id}/tokens/${trigger_datetime}`);
    notification.success({
        message: 'Task Activated',
        description: 'The task has been activated and will run shortly.',
        placement: 'bottomLeft',
    })
}

function makeCell(task, tok) {
    let this_task = tok.task_states[task];

    return (
        <Cell key={task}>
            <Popconfirm
                key="1"
                title={'Activate this task?'}
                okText={'Confirm'}
                cancelText={'Cancel'}
                okButtonProps={{size: 'normal'}}
                cancelButtonProps={{size: 'normal'}}
                onConfirm={() => activateToken(tok.trigger_datetime, this_task.task_id)}
                icon={<QuestionCircleOutlined style={{ color: geekblue[5] }}/>}
            >
                {iconForState(this_task)}    
            </Popconfirm>
        </Cell>
    );
}

function parseData(job_id, data) {
    let {tasks, tokens} = data;


    let columns = <tr>{
        [
            <td key="trigger_datetime">Trigger Datetime</td>
        ].concat(tasks.map(t => (
            <HeaderCell key={t}>{t}</HeaderCell>
        )))
    }</tr>;


    let rows = tokens.map(tok => (
        <Row key={tok.trigger_datetime}>{
            [
                <Cell key="trigger_datetime">
                    <Link to={`/jobs/${job_id}/tokens/${tok.trigger_datetime}`}>
                        {tok.trigger_datetime}
                    </Link>
                </Cell>
            ].concat(tasks.map(task => (
                makeCell(task, tok)
            )))
        }</Row>
    ));


    return { columns, rows };
}


class TaskGrid extends Component {
    constructor(props) {
        super(props);

        this.state = {
            loading: false,
            rows: [],
            columns: [],
        }
    }

    async fetchTokens() {
        const { id } = this.props;

        try {
            this.setState({
                loading: true
            });
            let resp = await axios.get(`/api/jobs/${id}/tokens-overview?limit=25`);

            let {rows, columns} = parseData(id, resp.data);

            this.setState({
                rows: rows,
                columns: columns,
                loading: false,
            });
        } catch(e) {
            this.setState({
                loading: false,
            });

            notification.error({
                message: 'Error fetching Tokens',
                description: e,
                placement: 'bottomLeft',
            });
        }
    }

    componentDidMount() {
        this.fetchTokens()

        this.interval = setInterval(() => this.fetchTokens(), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { id } = this.props;
        const { rows, columns, loading } = this.state;

        return (
            <Fragment>
                <table>
                    <thead>
                        {columns}
                    </thead>
                    <tbody>
                        {rows}
                    </tbody>
                </table>
            </Fragment>
        );
    }
}

export default TaskGrid;
