import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader, Row, Col, Statistic } from 'antd';
import { geekblue } from '@ant-design/colors';


import axios from 'axios';
import Moment from 'react-moment';

import Body from '../components/Body.jsx';
import State from '../components/State.jsx';
import RelDate from '../components/Date.jsx';

const { Content } = Layout;

function makeColumns() {
    return [
        {
            title: 'Task',
            key: 'task_id',
            render: (text, record) => (
                <Link to={`/jobs/${record.job_id}/tokens/${record.trigger_datetime}`}>
                    {record.project_name}/{record.job_name}/{record.task_name}
                </Link>
            ),
        },{
            title: 'Trigger Time',
            dataIndex: 'trigger_datetime',
            render: text => <RelDate>{text}</RelDate>,
        },{
            title: 'Queued Time',
            dataIndex: 'queued_datetime',
            render: text => <RelDate>{text}</RelDate>,
        },{
            title: 'Started Time',
            dataIndex: 'started_datetime',
            render: text => <RelDate>{text}</RelDate>,
        },{
            title: 'Finished Time',
            dataIndex: 'finish_datetime',
            render: text => (text && <RelDate>{text}</RelDate>),
        },{
            title: 'State',
            dataIndex: 'state',
            render: text => <State state={text} />,
        }
    ];
}


class Worker extends Component {
    constructor(props) {
        super(props);

        this.columns = makeColumns(props.match.params.id);

        this.state = {
            tasks: [],

        }
    }

    async fetchWorker(id, trigger_datetime) {
        try {
            let resp = await axios.get(`/api/workers/${id}`);
            this.setState({
                tasks: resp.data.tasks,
                last_seen_datetime: resp.data.last_seen_datetime,
                running_tasks: resp.data.running_tasks,
                total_tasks: resp.data.total_tasks,
            });
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        const { id } = this.props.match.params;

        this.fetchWorker(id);
        this.interval = setInterval(() => this.fetchWorker(id), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }


    render() {
        const { history, match } = this.props;
        const { id } = match.params;
        const { tasks, last_seen_datetime, running_tasks, total_tasks } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/workers">Workers</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/workers/${id}`}>{id}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <PageHeader
                            onBack={() => history.back()}
                            title={`Worker ${id}`}
                            subTitle={'A worker'}
                        />
                        <Row gutter={[16, 32]}>
                            <Col span={6}>
                                <Statistic title="Running Tasks"
                                    valueStyle={{color: geekblue[5]}}
                                    value={running_tasks} />
                            </Col>
                            <Col span={6}>
                                <Statistic title="Total Tasks"
                                    valueStyle={{color: geekblue[5]}}
                                    value={total_tasks} />
                            </Col>
                            <Col span={6}>
                                <Statistic title="Last Seen"
                                    value={last_seen_datetime}
                                    formatter={(val) => <Moment fromNow withTitle>{val}</Moment>}
                                    />
                            </Col>
                            <Col span={24} />
                        </Row>
                        <Table key="1" columns={this.columns} dataSource={tasks} />
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Worker;
