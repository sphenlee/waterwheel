import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Select, notification } from 'antd';
import { geekblue, lime, red, grey, yellow } from '@ant-design/colors';
import axios from 'axios';

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

function iconForState(tok) {
    if (tok === undefined) {
        return '';
    }

    let state = tok.state;

    if (state == 'active') {
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

/*    let columns = [
      {
        title: 'Trigger Time',
        dataIndex: 'trigger_datetime',
        key: 'trigger_datetime',
        render: (text, record) => (
                <Link to={`/jobs/${job_id}/tokens/${record.trigger_datetime}`}>
                    {text}
                </Link>
            )
      }
    ].concat(tasks.map(t => ({
        title: t,
        dataIndex: t,
        key: t + '.task_id',
        render: (text, record) => {
            if (text !== undefined) {
                return <State state={text.state} />;
            } else {
                return <div />;
            }
        }
    })));

    console.log('cols', columns);

    let rows = tokens.map(t => ({
        trigger_datetime: t.trigger_datetime,
        ...t.task_states,
    }));

    console.log('rows', rows);
*/
    let columns = [
        <td>Trigger Datetime</td>
    ].concat(tasks.map(t => (<td style={{writingMode: 'vertical-rl'}}>{t}</td>)));

    let style = {
        borderBottom: '1px solid #ddd'
    };

    let rows = tokens.map(tok => (
        <tr>{
            [
                <td style={style}>
                    <Link to={`/jobs/${job_id}/tokens/${tok.trigger_datetime}`}>
                        {tok.trigger_datetime}
                    </Link>
                </td>
            ].concat(tasks.map(t => (
                <td style={style}>{iconForState(tok.task_states[t])}</td>
            )))
        }</tr>
    ));


    return { columns, rows };
}


class TokenTable extends Component {
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
                {/*<Table columns={columns} dataSource={rows} loading={rows === []} pagination={{ position: ['bottomLeft']}}/>*/}
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

export default TokenTable;
