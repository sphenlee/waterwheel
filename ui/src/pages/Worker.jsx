import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader } from 'antd';


import axios from 'axios';
import Moment from 'react-moment';

import Body from '../components/Body.jsx';
import State from '../components/State.jsx';
import RelDate from '../components/Date.jsx';

const { Content } = Layout;

function makeColumns() {
    /*
    job_id: Uuid,
    job_name: String,
    project_id: Uuid,
    project_name: String,
    task_id: Uuid,
    task_name: String,
    trigger_datetime: DateTime<Utc>,
    queued_datetime: DateTime<Utc>,
    started_datetime: DateTime<Utc>,
    finish_datetime: Option<DateTime<Utc>>,
    state: String,
    */
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
            job: {},
            tokens: []
        }
    }

    async fetchTasks(id, trigger_datetime) {
        try {
            let resp = await axios.get(`/api/workers/${id}`);
            this.setState({
                tasks: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        const { id } = this.props.match.params;

        this.fetchTasks(id);
    }

    render() {
        const { history, match } = this.props;
        const { id } = match.params;
        const { tasks } = this.state;

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
                            onBack={() => history.replace(`/workers`)}
                            title={`Worker ${id}`}
                            subTitle={'A worker'}
                        />
                        <Table key="1" columns={this.columns} dataSource={tasks} />
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Worker;
