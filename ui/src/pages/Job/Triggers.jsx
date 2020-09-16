import React, { Component } from "react";
import { Link } from "react-router-dom";
import { Table, Typography, Space, Tooltip, Tag } from 'antd';
import axios from 'axios';
import Moment from 'react-moment';
import cronstrue from 'cronstrue';
import prettyMilliseconds from 'pretty-ms';

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
            title: 'Schedule',
            key: 'period',
            render: (text, record) => record.period ? <Period>{record.period}</Period> : <Cron>{record.cron}</Cron>,
        },{
            title: 'Start',
            dataIndex: 'start_datetime',
        },{
            title: 'Earliest',
            dataIndex: 'earliest_trigger_datetime',
        },{
            title: 'Latest',
            dataIndex: 'latest_trigger_datetime',
            render: text => (
                <Space>
                    <Text>{text}</Text>
                    <Text type="secondary">
                        <Moment fromNow>{text}</Moment>
                    </Text>
                </Space>)
        },{
            title: 'End',
            dataIndex: 'end_datetime',
            render: text => (text || <Text type="secondary">never</Text>),
        }
    ];
}


function Period(props) {
    let string = prettyMilliseconds(props.children * 1000);
    return <Tag>{string}</Tag>;
}


function Cron(props) {
    let desc;
    try {
        desc = cronstrue.toString(props.children);
    } catch(e) {
        desc = e; 
    }

    return (
        <Tooltip title={desc}>
            <Tag>{props.children}</Tag>
        </Tooltip>
    );
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
