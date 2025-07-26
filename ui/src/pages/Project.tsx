import React, { Component, Fragment } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { Avatar, Layout, Breadcrumb, Row, Col, Statistic, Badge, Tag, Spin, Table,
  Typography } from 'antd';
import { ColumnsType } from "antd/es/table";
import { geekblue, lime, red, grey, yellow, orange } from '@ant-design/colors';
import { PauseOutlined, PartitionOutlined } from '@ant-design/icons';
import { PageHeader } from '@ant-design/pro-components';

import axios from 'axios';

import Body from '../components/Body';
import { ProjectExtra, ProjectJob } from "../types/Project";
import { interval } from "../types/common";


const { Content } = Layout;

type ProjectProps = {};
type ProjectParams = {
    id: string;
};

type ProjectState = {
    proj?: ProjectExtra;
    jobs?: ProjectJob[];
};

function makeBadges(job: ProjectJob): React.ReactNode {
    return (<Fragment>
       <Badge count={job.waiting}
           style={{background: grey[7]}}
           overflowCount={999}
           title="Waiting"/>
       <Badge count={job.running}
           style={{background: geekblue[7]}}
           overflowCount={999}
           title="Running"/>
       <Badge count={job.success}
           style={{background: lime[7]}}
           overflowCount={999}
           title="Success"/>
       <Badge count={job.failure}
           style={{background: red[7]}}
           overflowCount={999}
           title="Failure"/>
       <Badge count={job.error}
           style={{background: orange[5]}}
           overflowCount={999}
           title="Error"/>
   </Fragment>);
}

function makeColumns(): ColumnsType<ProjectJob> {
    return [
        {
            title: 'Paused',
            dataIndex: 'paused',
            render: paused => (paused && <Tag color="warning" icon={<PauseOutlined />}>paused</Tag>),
        },{
             title: 'Status',
             dataIndex: 'job_id',
             render: (_, job) => makeBadges(job)
         },{
            title: '',
            dataIndex: 'job_id',
            render: _ => <Avatar icon={<PartitionOutlined />} shape="square"></Avatar>
        },{
            title: 'Name',
            dataIndex: 'name',
            render: (_, record) => (
                <Link to={`/jobs/${record.job_id}`}>
                    {record.name}
                </Link>)
        },{
            title: 'Description',
            dataIndex: 'description',
        },{
             title: 'Job ID',
             dataIndex: 'job_id',
             render: text => <Typography.Text type="secondary">{text}</Typography.Text>
         }
    ];
}

class Project extends Component<ProjectProps, ProjectState> {
    interval: interval;
    columns: ColumnsType<ProjectJob>;

    constructor(props: ProjectProps) {
        super(props);

        this.state = {};
        this.columns = makeColumns();
    }

    async fetchProject() {
        const {id} = useParams() as ProjectParams;
        
        try {
            let proj = await axios.get<ProjectExtra>(`/api/projects/${id}`);
            let jobs = await axios.get<ProjectJob[]>(`/api/projects/${id}/jobs`);
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
        const navigate = useNavigate();
        const { proj, jobs } = this.state;

        const content = proj ? (
            <>
                <PageHeader
                    onBack={() => navigate("/projects")}
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
                </Row>
                <Row>
                    <Col span={24}>
                        <Table rowKey="job_id"
                            columns={this.columns}
                            dataSource={jobs ?? []}
                            loading={jobs === null}
                            pagination={{position: ['bottomLeft']}}
                            />
                    </Col>
                </Row>
            </>
        ) : <Spin size="large" />;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/projects/${proj?.id}`}>{proj?.name || "..."}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>{content}</Body>
                </Content>
            </Layout>
        );
    }
}

export default Project;

