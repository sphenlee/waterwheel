import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Select, notification } from 'antd';
import { geekblue, lime, red, grey, yellow } from '@ant-design/colors';
import axios from 'axios';
import styled from 'styled-components';

import {
  CheckCircleOutlined,
  SyncOutlined,
  CloseCircleOutlined,
  ExclamationCircleOutlined,
  ClockCircleOutlined,
  MinusCircleOutlined,
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


function iconForState(tok) {
    if (tok === undefined) {
        return '';
    }

    let state = tok.state;

    if (state == 'active') {
        return <SyncOutlined style={{color: grey[5]}}/>;
    } else if (state == 'running') {
        return <SyncOutlined spin style={{color: geekblue[5]}}/>;
    } else if (state == 'waiting') {
        return <ClockCircleOutlined style={{color: grey[5]}}/>;
    } else if (state == 'success') {
        return <CheckCircleOutlined style={{color: lime[5]}}/>;
    } else if (state == 'failure') {
        return <CloseCircleOutlined style={{color: red[5]}}/>;
    } else {
        return '';
    }
}

function parseData(job_id, data) {
    let {tasks, tokens} = data;


    let columns = <tr>{[
        <td key="trigger_datetime">Trigger Datetime</td>
    ].concat(tasks.map(t => (<HeaderCell key={t}>{t}</HeaderCell>)))
}</tr>;


    let rows = tokens.map(tok => (
        <Row key={tok.trigger_datetime}>{
            [
                <Cell key="trigger_datetime">
                    <Link to={`/jobs/${job_id}/tokens/${tok.trigger_datetime}`}>
                        {tok.trigger_datetime}
                    </Link>
                </Cell>
            ].concat(tasks.map(t => (
                <Cell key={t}>
                    {iconForState(tok.task_states[t])}
                </Cell>
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
            let resp = await axios.get(`/api/jobs/${id}/tokens-overview`);

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
