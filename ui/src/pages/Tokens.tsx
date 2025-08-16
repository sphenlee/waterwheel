import React, { Component, Fragment, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { Table, Layout, Breadcrumb, Button, notification, Popconfirm,
        Row, Col, Drawer, Spin } from 'antd';
import { PageHeader } from '@ant-design/pro-components';

import { ExclamationCircleOutlined } from '@ant-design/icons';

import axios from 'axios';

import Body from '../components/Body';
import State from '../components/State';
import Graph from '../components/Graph';
import TokenRuns from './TokenRuns';
import { ColumnsType } from "antd/es/table";
import { JobExtra } from "../types/Job";
import { Token } from "../types/Token";
import { interval } from "../types/common";

const { Content } = Layout;

type TokensParams = {
    id: string;
    trigger_datetime: string;
};

type TokensState = {
    job?: JobExtra;
    tokens: Token[];
    drawer_task_id: string | null;
};

function makeColumns(job_id: string): ColumnsType<Token> {
    return [
        {
        title: 'Task',
        dataIndex: 'task_name',
        key: 'task_name',
        },{
        title: 'State',
        dataIndex: 'state',
        render: text => <State state={text} />,
        },/*{
        title: '',
        dataIndex: 'task_id',
        key: 'task_id',
        render: (text, record) => <ActivateToken
            type="default" size="small"
            task_id={record.task_id}
            trigger_datetime={record.trigger_datetime}
            />,
        },{
        title: '',
        dataIndex: 'task_id',
        key: 'task_id',
        render: (text, record) => <Button
            icon={<EllipsisOutlined/>}
            onClick={() => {
                this.drawerOpen(record);
            }}/>,
        }*/
    ];
}


function Tokens() {
    const { id, trigger_datetime } = useParams() as TokensParams;
    const [tokens, setTokens] = useState([] as Token[]);
    const [drawer_task_id, set_drawer_task_id] = useState<string|null>(null);
    const [job, setJob] = useState<JobExtra>();
    const navigate = useNavigate();
    

    const columns = makeColumns(id);

    async function fetchTokens(id: string, trigger_datetime: string) {
        try {
            let resp = await axios.get(`/api/jobs/${id}/tokens/${trigger_datetime}`);
            setTokens(resp.data);
        } catch(e) {
            console.log(e);
        }
    }

    async function fetchJob(id: string) {
        try {
            let resp = await axios.get<JobExtra>(`/api/jobs/${id}`);
            setJob(resp.data);
        } catch(e) {
            console.log(e);
        }
    }

    async function clearAllTokens() {
        if(!job) {
            console.warn(
                'Attempted to clear tokens before job could be loaded, allowing but job name will be misreported',
                {job_id: id},
            );
            return;
        }

        const { name } = job;

        try {
            let resp = await axios.delete(`/api/jobs/${id}/tokens/${trigger_datetime}`)
            notification.success({
                message: 'Tokens cleared',
                description: `Tokens for ${name} @ ${trigger_datetime} have been cleared`,
                placement: 'bottomLeft',
            })
        } catch(e) {
            console.log(e)
            notification.error({
                message: 'Error',
                description: 'Failed to clear tokens, see error console for details',
                placement: 'bottomLeft',
            })
        }
    }

    useEffect(() => {
        fetchJob(id);
        fetchTokens(id, trigger_datetime);

        const interval = setInterval(() => fetchTokens(id, trigger_datetime), 5000);

        return () => {
            clearInterval(interval);
        };
    }, []);
    
       

    const content = job ? (
        <>
            <PageHeader
                onBack={() => navigate(-1)}
                title={`${job.name} @ ${trigger_datetime}`}
                subTitle={job.description}
                extra={[
                    <Popconfirm
                        key="1"
                        title={'Clear all tokens for this trigger time?'}
                        okText={'Confirm'}
                        cancelText={'Cancel'}
                        okButtonProps={{size: 'middle', danger: true}}
                        cancelButtonProps={{size: 'middle'}}
                        onConfirm={() => clearAllTokens()}
                        icon={<ExclamationCircleOutlined />}
                    >
                        <Button danger>Clear</Button>
                    </Popconfirm>
                ]}
            />
            <Row>
                <Col span={12}>
                    <Table key="1" columns={columns} dataSource={tokens}
                        pagination={false} size={'small'}
                        onRow={(record, index) => ({
                            onClick: event => {
                                set_drawer_task_id(record.task_id);
                            }
                        })}
                        />
                </Col>
                <Col span={12}>
                    <Graph key="2" id={id} trigger_datetime={trigger_datetime} />
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
                    <Breadcrumb.Item><Link to={`/projects/${job?.project_id}`}>{job?.project}</Link></Breadcrumb.Item>
                    <Breadcrumb.Item><Link to={`/jobs/${id}`}>{job?.name}</Link></Breadcrumb.Item>
                    <Breadcrumb.Item><Link to={`/jobs/${id}/triggers/${trigger_datetime}`}>{trigger_datetime}</Link></Breadcrumb.Item>
                </Breadcrumb>
                <Body>{content}</Body>
            </Content>

            <Drawer
                    placement="bottom"
                    // size="large"
                    height={736} // todo - remove after upgrading
                    onClose={() => set_drawer_task_id(null)}
                    open={drawer_task_id !== null}>
                <TokenRuns
                    task_id={drawer_task_id}
                    trigger_datetime={trigger_datetime} />
            </Drawer>
        </Layout>
    );
}

export default Tokens;
