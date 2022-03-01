import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader, Button, notification, Badge} from 'antd';
import { geekblue, lime, red, grey, yellow } from '@ant-design/colors';
import axios from 'axios';

import Body from '../components/Body.jsx';
import State from '../components/State.jsx';
import { ColumnsType } from "antd/lib/table";

const { Content } = Layout;


function makeColumns(job_id) {
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

type TriggersProps = {
    history: any;
    match: any;
};
type TriggersState = {
    trigger: any;
};

class Triggers extends Component<TriggersProps, TriggersState> {
    columns: ColumnsType<any>;
    interval: NodeJS.Timeout;

    constructor(props: TriggersProps) {
        super(props);

        this.columns = makeColumns(props.match.params.job_id);

        this.state = {
            trigger: {}
        }
    }

    async fetchTrigger(job_id, trigger_id) {
        try {
            let resp = await axios.get(`/api/triggers/${trigger_id}`);
            this.setState({
                trigger: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        const { job_id, trigger_id } = this.props.match.params;

        this.fetchTrigger(job_id, trigger_id);

        this.interval = setInterval(() => this.fetchTrigger(job_id, trigger_id), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { history, match } = this.props;
        const { job_id, trigger_id } = match.params;
        const { trigger } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/projects/${trigger.project_id}`}>{trigger.project_name}</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/jobs/${job_id}`}>{trigger.job_name}</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/jobs/${job_id}/triggers/${trigger_id}`}>{trigger.trigger_name}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <PageHeader
                            onBack={() => history.goBack()}
                            title={trigger.trigger_name}
                            subTitle={`Trigger in ${trigger.job_name}`}
                        />
                        <Table columns={this.columns} dataSource={trigger.times} pagination={{position: ['bottomLeft']}}/>
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Triggers;
