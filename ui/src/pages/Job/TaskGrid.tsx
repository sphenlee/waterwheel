import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Select, notification, Popconfirm, Row, Button, DatePicker, Space, Col, Tooltip } from 'antd';
import { geekblue, lime, red, grey, orange, purple } from '@ant-design/colors';
import axios from 'axios';
import styled, { CSSProperties } from 'styled-components';

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
  StopOutlined,
  PlusSquareOutlined,
  HourglassOutlined,
} from '@ant-design/icons';
import { Token, TokenOverview, TokensRow, TokenState } from "../../types/Token";
import { datetime, uuid } from "../../types/common";
import { Moment } from "moment";
import { Task } from "../../types/Task";
import TokenRuns from "../TokenRuns";


const HeaderCell = styled.td`
    writing-mode: vertical-rl;
`;

const TCell = styled.td`
    transition: background 0.3s;
    padding-right: 7px;
    padding-left: 7px;
`;

const TRow = styled.tr`
    padding-bottom: 15px;
    transition: background 0.3s;
    &:hover {
        > td {
            background-color: ${geekblue[0]};
        }
    }
`;

function iconForState(task: TokenState) {
    if (task === undefined) {
        return <MinusOutlined style={{color: grey[5]}} />;
    }

    let state = task.state;
    let icon;

    if (state == 'active') {
        icon = <SyncOutlined style={{color: grey[5]}}/>;
    } else if (state == 'running') {
        icon = <SyncOutlined spin style={{color: geekblue[5]}}/>;
    } else if (state == 'waiting') {
        icon = <ClockCircleOutlined style={{color: grey[5]}}/>;
    } else if (state == 'success') {
        icon = <CheckCircleOutlined style={{color: lime[5]}}/>;
    } else if (state == 'failure') {
        icon = <CloseSquareOutlined style={{color: red[5]}}/>;
    } else if (state == 'timeout') {
        icon = <HourglassOutlined style={{color: orange[5]}}/>;
    } else if (state == 'error') {
        icon = <WarningOutlined style={{color: orange[5]}}/>;
    } else if (state == 'cancelled') {
        icon = <StopOutlined style={{color: grey[5]}} />;
    } else if (state == 'retry') {
        icon = <PlusSquareOutlined  style={{color: purple[6]}} />;
    } else {
        icon = 'invalid state?';
    }

    return <Tooltip title={state}>{icon}</Tooltip>;
}

async function activateToken(trigger_datetime: string, task_id: string) {
    await axios.put(`/api/tasks/${task_id}/tokens/${trigger_datetime}`, {});
    notification.success({
        message: 'Task Activated',
        description: 'The task has been activated and will run shortly.',
        placement: 'bottomRight',
    })
}


type TaskGridProps = {
    id: string;
};

type TaskGridState = {
    data: TokenOverview | null;
    limit: number;
    before: datetime | null;
    //last?: datetime;
    drawer_task_id: string | null;
    drawer_trigger_datetime: datetime | null;
}

class TaskGrid extends Component<TaskGridProps, TaskGridState> {
    interval: NodeJS.Timeout;

    constructor(props: TaskGridProps) {
        super(props);

        this.state = {
            data: null,
            limit: 25,
            before: null,
            drawer_task_id: null,
            drawer_trigger_datetime: null,
        }
    }

    parseData(job_id: string, data: TokenOverview) {
        let {tasks, tokens} = data;

        let columns = <tr>{
            [
                <td key="trigger_datetime">Trigger Datetime</td>
            ].concat(tasks.map(t => (
                <HeaderCell key={t}>{t}</HeaderCell>
            )))
        }</tr>;

        let rows = tokens.map(tok => {
            return <TRow key={tok.trigger_datetime}>{
                [
                    <TCell key="trigger_datetime">
                        <Link to={`/jobs/${job_id}/tokens/${tok.trigger_datetime}`}>
                            {tok.trigger_datetime}
                        </Link>
                    </TCell>
                ].concat(tasks.map(task => {
                    let this_task = tok.task_states[task];

                    let col_style: CSSProperties;
                    if (this_task && this_task.task_id === this.state.drawer_task_id
                    && tok.trigger_datetime == this.state.drawer_trigger_datetime) {
                        col_style = {
                            border: `solid 1px ${geekblue[7]}`,
                        };
                    } else {
                        col_style = {};
                    }

                    return (
                        <TCell key={task} style={col_style}>
                            <a onClick={() => this.drawerOpen(this_task.task_id, tok.trigger_datetime)}>
                                {iconForState(this_task)}    
                            </a>
                        </TCell>
                    );
                }))
            }</TRow>;
        });


        return { columns, rows };
    }


    gotoCurrent() {
        this.setState({
            before: null,
        });
    }

    onDatePicked(date: Moment | null) {
        this.setState({
            before: date && date.toISOString()
        });
    }

    drawerOpen(task_id: uuid, trigger_datetime: datetime) {
        this.setState({
            drawer_task_id: task_id,
            drawer_trigger_datetime: trigger_datetime,
        });
    }

    async fetchTokens() {
        const { id } = this.props;
        const { limit, before } = this.state;

        let params = {
            limit: limit,
            before: before,
        };

        let resp = await axios.get<TokenOverview>(`/api/jobs/${id}/tokens-overview`, {
                params: params
        });

        //let last = resp.data.tokens[resp.data.tokens.length - 1].trigger_datetime;
        
        this.setState({
            data: resp.data,
            //last: last,
        });
    }

    componentDidMount() {
        this.fetchTokens()

        // TODO - use a websocket to poll for token status changes
        this.interval = setInterval(() => this.fetchTokens(), 1000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { id }  = this.props;
        const { data, drawer_task_id, drawer_trigger_datetime } = this.state;

        if(!data) {
            return null;
        }

        const {rows, columns} = this.parseData(id, data);

        return (
            <Fragment>
                <Row>
                    <DatePicker onChange={(date) => this.onDatePicked(date)} />
                    <Space />
                    <Button onClick={() => this.gotoCurrent()} icon={<DoubleRightOutlined />}>
                        Latest
                    </Button>
                </Row>
                <Row>
                    <Col span={12}>
                        <table>
                            <thead>
                                {columns}
                            </thead>
                            <tbody>
                                {rows}
                            </tbody>
                        </table>
                    </Col>
                    <Col span={12}>
                        <TokenRuns
                            task_id={drawer_task_id}
                            trigger_datetime={drawer_trigger_datetime} />
                    </Col>
                </Row>


            </Fragment>
        );
    }
}

export default TaskGrid;
