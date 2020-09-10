import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Typography } from 'antd';
import axios from 'axios';

const { Text } = Typography;

function makeColumns(job) {
    return [
        {
            title: 'Name',
            dataIndex: 'trigger_name',
            render: (text, record) => (
                <Link to={`/jobs/${job.id}/triggers/${record.trigger_id}`}>
                    {text}
                </Link>
            ),
        },{
            title: 'Start',
            dataIndex: 'start_datetime',
        },{
            title: 'Earliest',
            dataIndex: 'earliest_trigger_datetime',
        },{
            title: 'Latest',
            dataIndex: 'latest_trigger_datetime',
        },{
            title: 'End',
            dataIndex: 'end_datetime',
            render: text => (text || <Text type="secondary">never</Text>),
        },{
            title: 'Schedule',
            key: 'period',
            render: (text, record) => (record.period || record.cron),
        }
    ];
}


class Triggers extends Component {
    constructor(props) {
        super(props);

        this.columns = makeColumns(props.job);

        this.state = {
            triggers: []
        }
    }

    async fetchTriggers(id) {
        let resp = await axios.get(`/api/jobs/${id}/triggers`);
        this.setState({
            triggers: resp.data,
        });
    }

    componentDidMount() {
        this.fetchTriggers(this.props.job.id);
    }

    render() {
        return (
            <Table rowKey={"trigger_id"} columns={this.columns} dataSource={this.state.triggers} />
        );
    }
}

export default Triggers;
