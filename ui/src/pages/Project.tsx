import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { List, Avatar, Layout, Breadcrumb, PageHeader, Row, Col, Statistic, Badge, Tag  } from 'antd';
import { geekblue, lime, red, grey, yellow, orange } from '@ant-design/colors';
import { PauseOutlined, PartitionOutlined } from '@ant-design/icons';
import axios from 'axios';

import Body from '../components/Body.jsx';

const { Content } = Layout;

type ProjectProps = {
    match: any;
    history: any;
};

type ProjectState = {
    proj: any;
    jobs: any;
};

class Project extends Component<ProjectProps, ProjectState> {
    interval: NodeJS.Timeout;

    constructor(props: ProjectProps) {
        super(props);

        this.state = {
            proj: {},
            jobs: null
        };
    }

    async fetchProject() {
        const {match} = this.props;
        
        try {
            let proj = await axios.get(`/api/projects/${match.params.id}`);
            let jobs = await axios.get(`/api/projects/${match.params.id}/jobs`);
            this.setState({
                proj: proj.data,
                jobs: jobs.data,
            });
        } catch(e) {
            console.log(e);
            this.setState({
                jobs: []
            });
        }
    }

    componentDidMount() {
        this.fetchProject()

        this.interval = setInterval(() => this.fetchProject(), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }


    render() {
        const { history } = this.props;
        const { proj } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/projects/${proj.id}`}>{proj.name || "..."}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <PageHeader
                            onBack={() => history.replace("/projects")}
                            title={proj.name}
                            subTitle={proj.description}
                        />
                        <Row gutter={[16, 32]}>
                            <Col span={4}>
                                <Statistic title="Jobs" value={proj.num_jobs} />
                            </Col>
                            <Col span={4}>
                                <Statistic title="Running Tasks"
                                    valueStyle={{color: geekblue[5]}}
                                    value={proj.running_tasks} />
                            </Col>
                            <Col span={4}>
                                <Statistic title="Waiting Tasks"
                                    valueStyle={{color: grey[5]}}
                                    value={proj.waiting_tasks} />
                            </Col>
                            <Col span={4}>
                                <Statistic title="Succeeded Tasks (last hour)"
                                    valueStyle={{color: lime[5]}}
                                    value={proj.succeeded_tasks_last_hour} />
                            </Col>
                            <Col span={4}>
                                <Statistic title="Failed Tasks (last hour)"
                                    valueStyle={{color: red[5]}}
                                    value={proj.failed_tasks_last_hour} />
                            </Col>
                            <Col span={4}>
                                <Statistic title="Error Tasks (last hour)"
                                    valueStyle={{color: orange[5]}}
                                    value={proj.error_tasks_last_hour} />
                            </Col>
                            <Col span={4} />
                            <Col span={24} />
                        </Row>
                        <Row>
                            <Col span={12}>
                                <List
                                    size="large"
                                    bordered={true}
                                    itemLayout="vertical"
                                    dataSource={this.state.jobs ?? []}
                                    loading={this.state.jobs === null}
                                    renderItem={(item: any) => (
                                        <List.Item
                                            key={item.job_id}
                                            actions={[
                                                <Fragment>
                                                    <Badge count={item.waiting}
                                                        style={{background: grey[7]}}
                                                        overflowCount={999}
                                                        title="Waiting"/>
                                                    <Badge count={item.running}
                                                        style={{background: geekblue[7]}}
                                                        overflowCount={999}
                                                        title="Running"/>
                                                    <Badge count={item.success}
                                                        style={{background: lime[7]}}
                                                        overflowCount={999}
                                                        title="Success"/>
                                                    <Badge count={item.failure}
                                                        style={{background: red[7]}}
                                                        overflowCount={999}
                                                        title="Failure"/>
                                                    <Badge count={item.error}
                                                        style={{background: orange[5]}}
                                                        overflowCount={999}
                                                        title="Error"/>
                                                </Fragment>,
                                                (item.paused &&
                                                    <Tag color="warning" icon={<PauseOutlined />}>paused</Tag>)
                                            ]}
                                        >
                                            <List.Item.Meta
                                                avatar={<Avatar icon={<PartitionOutlined />} shape="square"></Avatar>}
                                                title={<Link to={`/jobs/${item.job_id}`}>
                                                        {item.name}
                                                    </Link>}
                                                description={item.description}
                                            />

                                        </List.Item>
                                    )}
                                />
                            </Col>
                        </Row>
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Project;

