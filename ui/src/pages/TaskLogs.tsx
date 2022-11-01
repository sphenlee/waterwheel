import React, { Component } from "react";
import { Link, RouteComponentProps } from "react-router-dom";
import { Layout, Breadcrumb, PageHeader } from 'antd';

import Body from '../components/Body';
import Log from '../components/Log';

const { Content } = Layout;

type TaskLogsProps = RouteComponentProps<{
     task_run_id: string;
}>;

class TaskLogs extends Component<TaskLogsProps> {
  render() {
    const { history, match } = this.props;
    const { task_run_id } = match.params;

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
                onBack={() => history.goBack()}
                title={`Logs for ${task_run_id}`}
                subTitle={"TODO - include the project/job/task details here"}
            />
            <Log ws={`/api/task_runs/${task_run_id}/logs`} />
          </Body>
        </Content>
      </Layout>
    );
  }
}

export default TaskLogs;

