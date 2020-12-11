import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Select, notification } from 'antd';
import axios from 'axios';

import State from '../../components/State.jsx';

const { Option } = Select;

function parseData(job_id, data) {
    let {tasks, tokens} = data;

    let columns = [
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
        render: (text, record) => (<State state={text.state} />)
    })));

    console.log('cols', columns);

    let rows = tokens.map(t => ({
        trigger_datetime: t.trigger_datetime,
        ...t.task_states,
    }));

    console.log('rows', rows);


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
                <Table columns={columns} dataSource={rows} loading={rows === []} pagination={{ position: ['bottomLeft']}}/>
            </Fragment>
        );
    }
}

export default TokenTable;
