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
            key: 'trigger_name',
            render: (text, record) => (
                <Link to={`/jobs/${job.id}/triggers/${record.trigger_id}`}>
                    {text}
                </Link>
            ),
        },{
            title: 'Start',
            dataIndex: 'start_datetime',
            key: 'start_datetime',
            //render: text => <a>{text}</a>,
        },{
            title: 'Earliest',
            dataIndex: 'earliest_trigger_datetime',
            key: 'earliest_trigger_datetime',
            //render: text => <a>{text}</a>,
        },{
            title: 'Latest',
            dataIndex: 'latest_trigger_datetime',
            key: 'latest_trigger_datetime',
            //render: text => <a>{text}</a>,
        },{
            title: 'End',
            dataIndex: 'end_datetime',
            key: 'end_datetime',
            render: text => (text || <Text type="secondary">never</Text>),
        },{
            title: 'Period',
            dataIndex: 'period',
            key: 'period',
            //render: text => <a>{text}</a>,
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
            <Table columns={this.columns} dataSource={this.state.triggers} />
        );
    }
}

export default Triggers;
