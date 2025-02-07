import React, { Component, Fragment } from "react";
import { Link, RouteComponentProps } from "react-router-dom";
import { Layout, Breadcrumb, Tabs, Row, Col, Statistic, Spin, Tag } from 'antd';
import { PageHeader } from '@ant-design/pro-components';
import { geekblue, lime, red, grey, yellow, orange } from '@ant-design/colors';
import { PauseOutlined } from '@ant-design/icons';
import JSONPretty from 'react-json-pretty';
import axios from 'axios';

import Body from '../components/Body';
import TokenTable from './Job/TokenTable';
import Triggers from './Job/Triggers';
import Graph from '../components/Graph';
import TaskGrid from './Job/TaskGrid';
import Duration from './Job/Duration';
import { JobExtra } from "../types/Job";
import { interval } from "../types/common";

const { Content } = Layout;

type JobProps = RouteComponentProps<{
    id: string;
    tab: string;
}>;

type JobState = {
    job?: JobExtra;
}

class Job extends Component<JobProps, JobState> {
    interval: interval;

    constructor(props: JobProps) {
        super(props);

        this.state = {};
    }

    async fetchJob(id: string) {
        try {
            let resp = await axios.get<JobExtra>(`/api/jobs/${id}`);
            this.setState({job: resp.data});
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        const { id } = this.props.match.params;

        this.fetchJob(id)

        this.interval = setInterval(() => this.fetchJob(id), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { history, match } = this.props;
        const { job } = this.state;

        const job_id = match.params.id;
        const tab = match.params.tab || 'overview';

        const content = job ? (
            <>
                <PageHeader
                    onBack={() => history.goBack()}
                    title={job.name}
                    subTitle={job.description}
                    tags={job.paused ? <Tag color="warning" icon={<PauseOutlined />}>paused</Tag> : <></>}
                />
                <Tabs
                    activeKey={tab}
                    onChange={(activeKey) => history.replace(`/jobs/${job_id}/${activeKey}`)}
                    destroyInactiveTabPane={true}
                    items={[
                        {
                            label: "Overview",
                            key: "overview",
                            children: <Row gutter={[16, 32]}>
                                          <Col span={4}>
                                              <Statistic title="Running Tasks"
                                                  valueStyle={{color: geekblue[5]}}
                                                  value={job.active_tasks} />
                                          </Col>
                                          <Col span={4}>
                                              <Statistic title="Waiting Tasks"
                                                  valueStyle={{color: grey[5]}}
                                                  value={job.waiting_tasks} />
                                          </Col>
                                          <Col span={4}>
                                              <Statistic title="Succeeded Tasks (last hour)"
                                                  valueStyle={{color: lime[5]}}
                                                  value={job.succeeded_tasks_last_hour} />
                                          </Col>
                                          <Col span={4}>
                                              <Statistic title="Failed Tasks (last hour)"
                                                  valueStyle={{color: red[5]}}
                                                  value={job.failed_tasks_last_hour} />
                                          </Col>
                                          <Col span={4}>
                                              <Statistic title="Error Tasks (last hour)"
                                                  valueStyle={{color: orange[5]}}
                                                  value={job.error_tasks_last_hour} />
                                          </Col>
                                          <Col span={24}>
                                              <Graph id={job_id} />
                                          </Col>
                                      </Row>
                        },{
                            label: "Grid",
                            key: "grid",
                            children: <Spin spinning={!job}>
                                          <TaskGrid id={job_id} />
                                      </Spin>
                        },{
                          label: "Triggers",
                          key: "triggers",
                          children: <Triggers id={job_id} job={job} />
                      },
                      {
                          label: "Tokens",
                          key: "tokens",
                          children: <TokenTable id={job_id}/>
                      },
                      {
                          label: "Duration",
                          key: "duration",
                          children: <Duration id={job_id} />
                      },
                      {
                          label: "Definition",
                          key: "definition",
                          children: <JSONPretty data={job.raw_definition} />
                      }
                    ]}
                />
            </>
        ) : <Spin size="large" />;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/projects/${job?.project_id}`}>{job?.project ?? "..."}</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/jobs/${job_id}`}>{job?.name ?? "..."}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>{content}</Body>
                </Content>
            </Layout>
        );
    }
}

export default Job;

