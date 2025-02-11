import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb } from 'antd';
import axios from 'axios';

import Body from '../components/Body';
import WorkerStatus from '../components/WorkerStatus';
import RelDate from '../components/Date';
import { ColumnsType } from "antd/es/table";
import { SchedulerState } from "../types/Scheduler";
import { interval } from "../types/common";

const { Content } = Layout;

function makeColumns(): ColumnsType<SchedulerState> {
    return [
        {
            title: 'Id',
            dataIndex: 'uuid',
        },{
            title: 'Status',
            dataIndex: 'status',
            render: text => <WorkerStatus status={text} />,
        },{
            title: 'Queued Triggers',
            dataIndex: 'queued_triggers',
        },{
            title: 'Next Trigger',
            dataIndex: 'waiting_for_trigger_id',
            render: (text, record) => (
                // TODO - job ID should be in this URL
                <Link to={`/jobs/${record.waiting_for_trigger_job_id}/triggers/${text}`}>
                    {text}
                </Link>
            ),
        },{
            title: 'Last Seen',
            dataIndex: 'last_seen_datetime',
            render: text => <RelDate>{text}</RelDate>
        }
    ];
}

type SchedulersState = {
    schedulers: SchedulerState[]
    loading: boolean;
};

class Schedulers extends Component<{}, SchedulersState> {
    columns: ColumnsType<SchedulerState>;
    interval: interval;

    constructor(props: {}) {
        super(props);

        this.columns = makeColumns();

        this.state = {
            loading: false,
            schedulers: []
        };
    }

    async fetchSchedulers() {
        try {
            this.setState({
                loading: true
            });
            let resp = await axios.get<SchedulerState[]>('/api/schedulers');
            this.setState({
                loading: false,
                schedulers: resp.data
            });
        } catch(e) {
            console.log(e);
            this.setState({
                loading: false,
                schedulers:[]
            });
        }
    }

    componentDidMount() {
        this.fetchSchedulers()
        this.interval = setInterval(() => this.fetchSchedulers(), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { schedulers, loading } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/schedulers">Schedulers</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <Table rowKey="uuid"
                            columns={this.columns}
                            dataSource={schedulers}
                            loading={loading}
                            pagination={{position: ['bottomLeft']}}
                            />
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Schedulers;

