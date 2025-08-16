import React, { Component } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { Layout, Breadcrumb } from 'antd';
import { PageHeader } from '@ant-design/pro-components';

import Body from '../components/Body';
import Log from '../components/Log';

const { Content } = Layout;

type TaskLogsParams = {
     task_run_id: string;
};

function TaskLogs() {
  const navigate = useNavigate();
  const { task_run_id } = useParams() as TaskLogsParams;

  return (
    <Layout>
      <Content style={{padding: '50px'}}>
        <Breadcrumb style={{paddingBottom: '12px'}}>
          <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
          <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
          <Breadcrumb.Item><Link to="/projects">...</Link></Breadcrumb.Item>
          <Breadcrumb.Item><Link to="/projects">...</Link></Breadcrumb.Item>
        </Breadcrumb>
        <Body>
          <PageHeader
              onBack={() => navigate(-1)}
              title={`Logs for ${task_run_id}`}
              subTitle={"TODO - include the project/job/task details here"}
          />
          <Log ws={`/api/task_runs/${task_run_id}/logs`} />
        </Body>
      </Content>
    </Layout>
  );
}

export default TaskLogs;

