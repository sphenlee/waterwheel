import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { List, Avatar, Layout, Breadcrumb, PageHeader, Collapse, Tabs, Row, Col, Statistic, Spin, Tag } from 'antd';
import { geekblue, lime, red, grey, yellow } from '@ant-design/colors';
import { PauseOutlined } from '@ant-design/icons';
import JSONPretty from 'react-json-pretty';
import styled from 'styled-components';
import axios from 'axios';

import Body from '../components/Body.jsx';
import TokenTable from './Job/TokenTable.jsx';
import Triggers from './Job/Triggers.jsx';
import Graph from '../components/Graph.jsx';
import TaskGrid from './Job/TaskGrid.jsx';

const { Content } = Layout;


class Job extends Component {
    constructor(props) {
        super(props);

        this.state = {
            loading: true,
            job: {},
            tokens: []
        };
    }

    async fetchJob(id) {
        try {
            this.setState({
                loading: true,
            });
            let resp = await axios.get(`/api/jobs/${id}`);
            this.setState({
                job: resp.data,
                loading: false,
            });
        } catch(e) {
            console.log(e);
        }
    }

    // componentDidMount() {
    //     this.fetchJob();
    // }

    componentDidMount() {
        const { id } = this.props.match.params;

        this.fetchJob(id)

        this.interval = setInterval(() => this.fetchJob(id), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { history } = this.props;
        const { job, loading } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/projects/${job.project_id}`}>{job.project || "..."}</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/jobs/${job.id}`}>{job.name || "..."}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <PageHeader
                            onBack={() => history.replace(`/projects/${job.project_id}`)}
                            title={job.name}
                            subTitle={job.description}
                            tags={job.paused && <Tag color="warning" icon={<PauseOutlined />}>paused</Tag>}
                        />
                        <Tabs>
                            <Tabs.TabPane tab="Overview" key="1">
                                <Row gutter={[16, 32]}>
                                    <Col span={6}>
                                        <Statistic title="Active Tasks"
                                            valueStyle={{color: geekblue[5]}}
                                            value={job.active_tasks} />
                                    </Col>
                                    <Col span={6}>
                                        <Statistic title="Succeeded Tasks (last hour)"
                                            valueStyle={{color: lime[5]}}
                                            value={job.succeeded_tasks_last_hour} />
                                    </Col>
                                    <Col span={6}>
                                        <Statistic title="Failed Tasks (last hour)"
                                            valueStyle={{color: red[5]}}
                                            value={job.failed_tasks_last_hour} />
                                    </Col>
                                    <Col span={24}>
                                        <Graph id={job.id} />
                                    </Col>
                                </Row>
                            </Tabs.TabPane>
                            <Tabs.TabPane tab="Grid" key="2">
                                <Spin spinning={loading}>
                                    <TaskGrid id={job.id} />
                                </Spin>
                            </Tabs.TabPane>
                            <Tabs.TabPane tab="Triggers" key="3">
                                <Triggers id={job.id} job={job}/>
                            </Tabs.TabPane>
                            <Tabs.TabPane tab="Tokens" key="4">
                                <TokenTable id={job.id}/>
                            </Tabs.TabPane>
                            <Tabs.TabPane tab="Definition" key="5">
                                <JSONPretty data={job.raw_definition} />
                            </Tabs.TabPane>
                        </Tabs>
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Job;

