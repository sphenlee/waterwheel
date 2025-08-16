import React, { Component, Fragment, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { Table, Layout, Breadcrumb, Badge, Spin} from 'antd';
import { PageHeader } from '@ant-design/pro-components';
import { geekblue, lime, red, grey, yellow } from '@ant-design/colors';

import axios from 'axios';

import Body from '../components/Body';
import { ColumnsType } from "antd/es/table";
import { Trigger, TriggerTime } from "../types/Job";
import { interval } from "../types/common";

const { Content } = Layout;


function makeColumns(job_id: string): ColumnsType<TriggerTime> {
    return [
      {
        title: 'Trigger Time',
        dataIndex: 'trigger_datetime',
        render: (text, record) => (
                <Link to={`/jobs/${job_id}/tokens/${record.trigger_datetime}`}>
                    {text}
                </Link>
            ),
        width: 300,
      },{
        title: 'Tasks',
        key: 'status',
        render: (text, record) => (
                <Fragment>
                    <Badge count={record.waiting} style={{background: grey[7]}} title="Waiting"/>
                    <Badge count={record.running} style={{background: geekblue[7]}} title="Active"/>
                    <Badge count={record.success} style={{background: lime[7]}} title="Success"/>
                    <Badge count={record.failure} style={{background: red[7]}} title="Failure"/>
                </Fragment>
            )

      }
    ];
}

type TriggersParams = {
    job_id: string;
    trigger_id: string;
};

function Triggers() {
    const [trigger, setTrigger] = useState<Trigger>();
    const { job_id, trigger_id } = useParams() as TriggersParams;
    const navigate = useNavigate();

    const columns = makeColumns(job_id);
    
    async function fetchTrigger() {
        try {
            let resp = await axios.get<Trigger>(`/api/triggers/${trigger_id}`);
            setTrigger(resp.data);
        } catch(e) {
            console.log(e);
        }
    }

    useEffect(() => {
        fetchTrigger();
        const interval = setInterval(fetchTrigger, 5000);
        return () => clearInterval(interval);
    }, []);     
        

    const content = trigger ? (
        <>
            <PageHeader
                onBack={() => navigate(-1)}
                title={trigger.trigger_name}
                subTitle={`Trigger in ${trigger.job_name}`}
            />
            <Table<TriggerTime> columns={columns} dataSource={trigger.times} pagination={{position: ['bottomLeft']}}/>
        </>
    ) : <Spin size="large" />;

    return (
        <Layout>
            <Content style={{padding: '50px'}}>
                <Breadcrumb style={{paddingBottom: '12px'}}>
                    <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                    <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                    <Breadcrumb.Item><Link to={`/projects/${trigger?.project_id}`}>{trigger?.project_name}</Link></Breadcrumb.Item>
                    <Breadcrumb.Item><Link to={`/jobs/${job_id}`}>{trigger?.job_name}</Link></Breadcrumb.Item>
                    <Breadcrumb.Item><Link to={`/jobs/${job_id}/triggers/${trigger_id}`}>{trigger?.trigger_name}</Link></Breadcrumb.Item>
                </Breadcrumb>
                <Body>{content}</Body>
            </Content>
        </Layout>
    );
}

export default Triggers;
