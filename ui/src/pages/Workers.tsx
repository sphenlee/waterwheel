import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb } from 'antd';
import axios from 'axios';
import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';

dayjs.extend(relativeTime);

import Body from '../components/Body';
import WorkerStatus from '../components/WorkerStatus';
import { ColumnsType } from "antd/es/table";
import { WorkerState } from "../types/Worker";
import { interval } from "../types/common";

const { Content } = Layout;

function makeColumns(): ColumnsType<WorkerState> {
    return [
        {
            title: 'Id',
            dataIndex: 'uuid',
            render: (text, record) => (
                <Link to={`/workers/${record.uuid}`}>
                    {text}
                </Link>
            ),
        },{
            title: 'Status',
            dataIndex: 'status',
            render: text => <WorkerStatus status={text} />,
        },{
            title: 'Running Tasks',
            dataIndex: 'running_tasks',
        },{
            title: 'Total Tasks',
            dataIndex: 'total_tasks',
        },/*{
            title: 'UI Address',
            dataIndex: 'addr',
            render: text => <a href={`http://${text}`}>{text}</a>,
        },*/{
            title: 'Last Seen',
            dataIndex: 'last_seen_datetime',
            render: text => dayjs(text).fromNow()
        }
    ];
}

type WorkersState = {
    workers: WorkerState[];
    loading: boolean;
};

class Workers extends Component<{}, WorkersState> {
    columns: ColumnsType<WorkerState>;
    interval: interval;

    constructor(props: {}) {
        super(props);

        this.columns = makeColumns();

        this.state = {
            loading: false,
            workers: []
        };
    }

    async fetchWorkers() {
        try {
            this.setState({
                loading: true
            });
            let resp = await axios.get<WorkerState[]>('/api/workers');
            this.setState({
                loading: false,
                workers: resp.data
            });
        } catch(e) {
            console.log(e);
            this.setState({
                loading: false,
                workers:[]
            });
        }
    }

    componentDidMount() {
        this.fetchWorkers()
        this.interval = setInterval(() => this.fetchWorkers(), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { workers, loading } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/workers">Workers</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <Table rowKey="uuid"
                            columns={this.columns}
                            dataSource={workers}
                            loading={loading}
                            pagination={{position: ['bottomLeft']}}
                            />
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Workers;

