import React, { Component } from "react";
import { Link } from "react-router-dom";
import { Table, Typography, Space, Tooltip, Tag } from 'antd';
import axios from 'axios';
import Moment from 'react-moment';
import cronstrue from 'cronstrue';
import prettyMilliseconds from 'pretty-ms';
import { ColumnsType } from "antd/lib/table";
import { Job, JobTrigger } from "../../types/Job";

const { Text } = Typography;

function makeColumns(job: Job): ColumnsType<JobTrigger> {
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
            render: (text, record) => record.period ? <Period period={record.period} /> : <Cron cron={record.cron ?? ''} />,
        },{
            title: 'Offset',
            dataIndex: 'trigger_offset',
            render: text => text ? <Period period={text} /> : ""
        },{
            title: 'Catchup',
            dataIndex: 'catchup',
        },{
            title: 'Start',
            dataIndex: 'start_datetime',
        },{
            title: 'Earliest',
            dataIndex: 'earliest_trigger_datetime',
        },{
            title: 'Latest',
            dataIndex: 'latest_trigger_datetime',
            render: text => (text ?
                <Space>
                    {text}
                    <Text type="secondary">
                        <Moment fromNow>{text}</Moment>
                    </Text>
                </Space>
                :  <Text type="secondary">never</Text>
                )
        },{
            title: 'End',
            dataIndex: 'end_datetime',
            render: text => (text || <Text type="secondary">never</Text>),
        }
    ];
}


function Period(props: {period: number}) {
    let string = prettyMilliseconds(props.period * 1000);
    return <Tag>{string}</Tag>;
}


function Cron(props: {cron: string}) {
    let desc;
    try {
        desc = cronstrue.toString(props.cron);
    } catch(e) {
        desc = e; 
    }

    return (
        <Tooltip title={desc}>
            <Tag>{props.cron}</Tag>
        </Tooltip>
    );
}


type TriggersProps = {
    id: string;
    job: Job;
};
type TriggersState = {
    triggers: JobTrigger[];
};

class Triggers extends Component<TriggersProps, TriggersState> {
    columns: ColumnsType<JobTrigger>;

    constructor(props: TriggersProps) {
        super(props);

        this.columns = makeColumns(props.job);

        this.state = {
            triggers: []
        }
    }

    async fetchTriggers(id: string) {
        let resp = await axios.get<JobTrigger[]>(`/api/jobs/${id}/triggers`);
        this.setState({
            triggers: resp.data,
        });
    }

    componentDidMount() {
        this.fetchTriggers(this.props.id);
    }

    render() {
        return (
            <Table rowKey={"trigger_id"} columns={this.columns} dataSource={this.state.triggers} pagination={{position: ['bottomLeft']}} />
        );
    }
}

export default Triggers;
