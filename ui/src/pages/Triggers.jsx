import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader, Button, notification } from 'antd';
import axios from 'axios';

import Body from '../components/Body.jsx';
import State from '../components/State.jsx';

const { Content } = Layout;


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
      }
    ];
}


class Triggers extends Component {
    constructor(props) {
        super(props);

        this.columns = makeColumns(props.match.params.job_id);

        this.state = {
            trigger: {},
            times: []
        }
    }

    async fetchTimes(trigger_id) {
        try {
            let resp = await axios.get(`/api/triggers/${trigger_id}`);
            this.setState({
                times: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    async fetchTrigger(job_id, trigger_id) {
        try {
            let resp = await axios.get(`/api/jobs/${job_id}/triggers/${trigger_id}`);
            this.setState({
                trigger: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        const { job_id, trigger_id } = this.props.match.params;

        this.fetchTrigger(job_id, trigger_id);
        this.fetchTimes(trigger_id);

        this.interval = setInterval(() => this.fetchTimes(trigger_id), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { history, match } = this.props;
        const { job_id, trigger_id } = match.params;
        const { trigger, times } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/projects/${trigger.project_id}`}>{trigger.project_name}</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/jobs/${job_id}`}>{trigger.job_name}</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/jobs/${job_id}/triggers/${trigger_id}`}>{trigger.trigger_name}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <PageHeader
                            onBack={() => history.replace(`/jobs/${job_id}`)}
                            title={trigger.trigger_name}
                            subTitle={`Trigger in ${trigger.job_name}`}
                        />
                        <Table columns={this.columns} dataSource={times} />
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Triggers;
