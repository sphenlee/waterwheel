import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Select, notification } from 'antd';
import axios from 'axios';

import State from '../../components/State.jsx';

const { Option } = Select;

function makeColumns(job_id) {
    return [
      {
        title: 'Trigger Time',
        dataIndex: 'trigger_datetime',
        key: 'trigger_datetime',
        render: (text, record) => (
                <Link to={`/jobs/${job_id}/tokens/${record.trigger_datetime}`}>
                    {text}
                </Link>
            )
      },{
        title: 'Task',
        dataIndex: 'task_name',
        key: 'task_name',
        render: text => text,
      },{
        title: 'Count',
        dataIndex: 'count',
        key: 'count',
        render: text => text,
      },{
        title: 'Threshold',
        dataIndex: 'threshold',
        key: 'threshold',
        render: text => text,
      },{
        title: 'State',
        dataIndex: 'state',
        key: 'state',
        render: text => <State state={text} />,
      }
    ];
}


class TokenTable extends Component {
    constructor(props) {
        super(props);

        this.columns = makeColumns(props.id);

        this.state = {
            filter: ['active'],
            loading: false,
            tokens: []
        }
    }

    async fetchTokens(id) {
        try {
            let filter = this.state.filter.concat(',');
            this.setState({
                loading: true
            });
            let resp = await axios.get(`/api/jobs/${id}/tokens?state=${filter}`);
            this.setState({
                tokens: resp.data,
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
        const { id } = this.props;

        this.fetchTokens(id)

        this.interval = setInterval(() => this.fetchTokens(id), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { id } = this.props;
        const { tokens, loading } = this.state;

        return (
            <Fragment>
                <Select
                  mode="multiple"
                  defaultValue={["active", "running"]}
                  style={{ width: 350 }}
                  onChange={(value) => {
                    this.setState({
                        filter: value
                    }, () => {
                        this.fetchTokens(id);    
                    });
                  }}
                >
                    <Option value="active">Active</Option>
                    <Option value="running">Running</Option>
                    <Option value="success">Success</Option>
                    <Option value="failure">Failure</Option>
                    <Option value="waiting">Waiting</Option>
                </Select>

                <Table columns={this.columns} dataSource={tokens} loading={loading} pagination={{position: ['bottomLeft']}}/>
            </Fragment>
        );
    }
}

export default TokenTable;
