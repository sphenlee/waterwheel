import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Select, notification, Popconfirm, Row, Button } from 'antd';
import { geekblue, lime, red, grey, orange } from '@ant-design/colors';
import axios from 'axios';
import styled from 'styled-components';

import {
  CheckCircleOutlined,
  SyncOutlined,
  CloseSquareOutlined,
  ClockCircleOutlined,
  MinusOutlined,
  WarningOutlined,
  QuestionCircleOutlined,
  LeftOutlined,
  DoubleRightOutlined,
} from '@ant-design/icons';


const HeaderCell = styled.td`
    writing-mode: vertical-rl;
`;

const TCell = styled.td`
    border-bottom: 1px solid #ddd;
    padding-right: 15px;
`;

const TRow = styled.tr`
    padding-bottom: 15px;
    transition: background 0.3s;
    &:hover {
        > td {
            background-color: #f8f8f8;
        }
    }
`;


function iconForState(task) {
    if (task === undefined) {
        return <MinusOutlined style={{color: grey[0]}} />;
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
        return 'invalid state?';
    }
}

async function activateToken(trigger_datetime, task_id) {
    await axios.put(`/api/tasks/${task_id}/tokens/${trigger_datetime}`);
    notification.success({
        message: 'Task Activated',
        description: 'The task has been activated and will run shortly.',
        placement: 'bottomRight',
    })
}

function makeCell(task, tok) {
    let this_task = tok.task_states[task];

    return (
        <TCell key={task}>
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
        </TCell>
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
        <TRow key={tok.trigger_datetime}>{
            [
                <TCell key="trigger_datetime">
                    <Link to={`/jobs/${job_id}/tokens/${tok.trigger_datetime}`}>
                        {tok.trigger_datetime}
                    </Link>
                </TCell>
            ].concat(tasks.map(task => (
                makeCell(task, tok)
            )))
        }</TRow>
    ));


    return { columns, rows };
}


class TaskGrid extends Component {
    constructor(props) {
        super(props);

        this.state = {
            data: null,
            limit: 25,
            before: null,
        }
    }

    previous() {
        this.setState((state) => ({
            before: state.last,
        }));
    }

    current() {
        this.setState({
            before: null,
        });
    }

    async fetchTokens() {
        const { id } = this.props;
        const { limit, before } = this.state;

        let params = {
            limit: limit,
            before: before,
        };

        let resp = await axios.get(`/api/jobs/${id}/tokens-overview`, {
                params: params
        });

        let last = resp.data.tokens[resp.data.tokens.length - 1].trigger_datetime;
        
        this.setState({
            data: resp.data,
            last: last,
        });
    }

    componentDidMount() {
        this.fetchTokens()

        // TODO - change back to 5s!
        // TODO - use a websocket to poll for token status changes
        this.interval = setInterval(() => this.fetchTokens(), 500);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { id }  = this.props;
        const { data } = this.state;

        if(!data) {
            return null;
        }

        const {rows, columns} = parseData(id, data);

        return (
            <Fragment>
                <Row>
                    <Button>
                        <LeftOutlined onClick={() => this.previous()}/>
                    </Button>
                    <Button>
                        <DoubleRightOutlined onClick={() => this.current()}/>
                    </Button>
                </Row>
                <Row>
                    <table>
                        <thead>
                            {columns}
                        </thead>
                        <tbody>
                            {rows}
                        </tbody>
                    </table>
                </Row>
            </Fragment>
        );
    }
}

export default TaskGrid;
