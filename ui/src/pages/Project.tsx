import React, { Component, Fragment, useEffect, useState } from "react";
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
import { JobExtra } from "../types/Job";


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

const columns = makeColumns();

function Project() {
    const [proj, setProj] = useState({} as ProjectExtra);
    const [jobs, setJobs] = useState([] as ProjectJob[])
    const {id} = useParams() as ProjectParams;
    const navigate = useNavigate();

    async function fetchProject() {        
        try {
            let proj = await axios.get<ProjectExtra>(`/api/projects/${id}`);
            setProj(proj.data);
            let jobs = await axios.get<ProjectJob[]>(`/api/projects/${id}/jobs`);
            setJobs(jobs.data);
        } catch(e) {
            console.log(e);
            setJobs([]);
        }
    }

    useEffect(() => {
        fetchProject();
        
        const interval = setInterval(() => fetchProject(), 5000);

        return () => clearInterval(interval);
    }, []);

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
                        columns={columns}
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

export default Project;

