import React, { Component, Fragment } from "react";
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

type TokensProps = {};
type TokensParams = {
    id: string;
    trigger_datetime: string;
};

type TokensState = {
    job?: JobExtra;
    tokens: Token[];
    drawer_task_id: string | null;
};

class Tokens extends Component<TokensProps, TokensState> {
    interval: interval;
    columns: ColumnsType<Token>;

    constructor(props: TokensProps) {
        super(props);

        const { id } = useParams() as TokensParams;

        this.columns = this.makeColumns(id!);

        this.state = {
            tokens: [],
            drawer_task_id: null,
        }
    }

    makeColumns(job_id: string): ColumnsType<Token> {
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

    async fetchTokens(id: string, trigger_datetime: string) {
        try {
            let resp = await axios.get(`/api/jobs/${id}/tokens/${trigger_datetime}`);
            this.setState({
                tokens: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    async fetchJob(id: string) {
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
        const {id, trigger_datetime} = useParams() as TokensParams;

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

    drawerOpen(record: Token) {
        this.setState({
            drawer_task_id: record.task_id
        });
    }

    componentDidMount() {
        const {id, trigger_datetime} = useParams() as TokensParams;

        this.fetchJob(id);
        this.fetchTokens(id, trigger_datetime);

        this.interval = setInterval(() => this.fetchTokens(id, trigger_datetime), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const navigate = useNavigate();
        const {id, trigger_datetime} = useParams() as TokensParams;
        const { job, tokens, drawer_task_id } = this.state;


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
                            pagination={false} size={'small'}
                            onRow={(record, index) => ({
                                onClick: event => {
                                    this.drawerOpen(record);
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
                        onClose={() => this.drawerClose()}
                        open={drawer_task_id !== null}>
                    <TokenRuns
                        task_id={drawer_task_id}
                        trigger_datetime={trigger_datetime} />
                </Drawer>
            </Layout>
        );
    }
}

export default Tokens;
