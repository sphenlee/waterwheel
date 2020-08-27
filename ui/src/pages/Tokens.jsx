import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader } from 'antd';
import styled from 'styled-components';
import axios from 'axios';

const { Content } = Layout;

const Body = styled.div`
    padding: 24px;
    background: #fff;
`;


function makeColumns(job_id) {
    return [
      /*{
        title: 'Trigger Time',
        dataIndex: 'trigger_datetime',
        key: 'trigger_datetime',
        render: (text, record) => (
                <Link to={`/jobs/${job_id}/tokens/${record.trigger_datetime}`}>
                    {text}
                </Link>
            )
      },*/{
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
        render: text => text,
      }
    ];
}


class Tokens extends Component {
    constructor(props) {
        super(props);

        this.columns = makeColumns(props.match.params.id);

        this.state = {
            job: {},
            tokens: []
        }
    }

    async fetchTokens(id, trigger_datetime) {
        try {
            let resp = await axios.get(`/api/jobs/${id}/tokens/${trigger_datetime}`);
            this.setState({
                tokens: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    async fetchJob(id) {
        try {
            let resp = await axios.get(`/api/jobs/${id}`);
            this.setState({
                job: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        const {id, trigger_datetime} = this.props.match.params;

        this.fetchJob(id);
        this.fetchTokens(id, trigger_datetime);

        this.interval = setInterval(() => this.fetchTokens(id, trigger_datetime), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { history, match } = this.props;
        const {id, trigger_datetime} = match.params;
        const { job, tokens } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/projects/${job.project_id}`}>{job.project}</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/jobs/${id}`}>{job.name}</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/jobs/${id}/triggers/${trigger_datetime}`}>{trigger_datetime}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <PageHeader
                            onBack={() => history.replace(`/jobs/${id}`)}
                            title={job.name}
                            subTitle={job.description}
                        />
                        <Table columns={this.columns} dataSource={tokens} />
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Tokens;
