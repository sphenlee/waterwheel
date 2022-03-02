import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader, Button, notification, Popconfirm,
        Row, Col, Drawer, Spin } from 'antd';

import { ExclamationCircleOutlined, EllipsisOutlined } from '@ant-design/icons';

import axios from 'axios';

import Body from '../components/Body';
import State from '../components/State';
import Graph from '../components/Graph';
import ActivateToken from '../components/ActivateToken';
import TokenRuns from './TokenRuns';
import { ColumnsType } from "antd/lib/table";
import { JobExtra } from "../types/Job";
import { Token } from "../types/Token";

const { Content } = Layout;

type TokensProps = {
    history: any;
    match: any;
};

type TokensState = {
    job?: JobExtra;
    tokens: Token[];
    drawer_task_id: string | null;
};

class Tokens extends Component<TokensProps, TokensState> {
    interval: NodeJS.Timeout;
    columns: ColumnsType<Token>;

    constructor(props: TokensProps) {
        super(props);

        this.columns = this.makeColumns(props.match.params.id);

        this.state = {
            tokens: [],
            drawer_task_id: null,
        }
    }

    makeColumns(job_id): ColumnsType<Token> {
        return [
          {
            title: 'Task',
            dataIndex: 'task_name',
            key: 'task_name',
          },{
            title: 'State',
            dataIndex: 'state',
            render: text => <State state={text} />,
          },{
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
                    this.drawOpen(record);
                }}/>,
          }
        ];
    }

    async fetchTokens(id, trigger_datetime) {
        try {
            let resp = await axios.get(`/api/jobs/${id}/tokens/${trigger_datetime}`);
            this.setState({
                tokens: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    async fetchJob(id) {
        try {
            let resp = await axios.get<JobExtra>(`/api/jobs/${id}`);
            this.setState({
                job: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    async clearAllTokens() {
        const {id, trigger_datetime} = this.props.match.params;

        if(!this.state.job) {
            console.warn(
                'Attempted to clear tokens before job could be loaded, allowing but job name will be misreported',
                {job_id: id},
            );
            return;
        }

        const { name } = this.state.job;

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

    drawerClose() {
        this.setState({
            drawer_task_id: null
        });
    }

    drawOpen(record: Token) {
        console.log(record);
        this.setState({
            drawer_task_id: record.task_id
        });
    }

    componentDidMount() {
        const {id, trigger_datetime} = this.props.match.params;

        this.fetchJob(id);
        this.fetchTokens(id, trigger_datetime);

        this.interval = setInterval(() => this.fetchTokens(id, trigger_datetime), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { history, match } = this.props;
        const {id, trigger_datetime} = match.params;
        const { job, tokens, drawer_task_id } = this.state;

        const content = job ? (
            <>
                <PageHeader
                    onBack={() => history.goBack()}
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
                            onConfirm={() => this.clearAllTokens()}
                            icon={<ExclamationCircleOutlined />}
                        >
                            <Button danger>Clear</Button>
                        </Popconfirm>
                    ]}
                />
                <Row>
                    <Col span={12}>
                        <Table key="1" columns={this.columns} dataSource={tokens}
                            pagination={{position: ['bottomLeft']}}
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

                <TokenRuns
                        task_id={drawer_task_id ?? ''}
                        trigger_datetime={trigger_datetime}
                        onClose={() => this.drawerClose()}
                        visible={drawer_task_id !== null} />
            </Layout>
        );
    }
}

export default Tokens;
