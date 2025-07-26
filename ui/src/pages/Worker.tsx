import React, { Component, Fragment } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { Table, Layout, Breadcrumb, Row, Col, Statistic,
    Descriptions, Button, Select, Spin } from 'antd';
import { PageHeader } from '@ant-design/pro-components';
import { geekblue } from '@ant-design/colors';
import { RightOutlined, DownOutlined } from "@ant-design/icons";
import { ColumnsType } from "antd/es/table";

const { Option } = Select;

import axios from 'axios';
import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';
dayjs.extend(relativeTime);


import Body from '../components/Body';
import State from '../components/State';
import RelDate from '../components/Date';
import WorkerStatus from '../components/WorkerStatus';
import { Worker as WorkerType, WorkerTask } from '../types/Worker';


const { Content } = Layout;

const defaultFilter = ["running"];

function makeColumns(): ColumnsType<WorkerTask> {
    return [
        {
            title: 'Task',
            key: 'task_id',
            render: (text, record) => (
                <Link to={`/jobs/${record.job_id}/tokens/${record.trigger_datetime}`}>
                    {record.project_name}/{record.job_name}/{record.task_name}
                </Link>
            ),
        },{
            title: 'Trigger Time',
            dataIndex: 'trigger_datetime',
            render: text => <RelDate>{text}</RelDate>,
        },{
            title: 'State',
            dataIndex: 'state',
            render: text => <State state={text} />,
        }
    ];
}


function expandedRowRender(record: WorkerTask) {
    return (
        <Descriptions
                size="small"
                bordered
                labelStyle={{
                    fontWeight: "bold"
                }}
                contentStyle={{
                    background: "#fff"
                }}>
            <Descriptions.Item label="Task Run Id">
                {record.task_run_id}
            </Descriptions.Item>
            <Descriptions.Item label="Logs">
                <Link to={`/logs/${record.task_run_id}`}>
                    logs
                </Link>
            </Descriptions.Item>
            {/*<Descriptions.Item label="Task">
                <Link to={`/jobs/${record.job_id}/tokens/${record.trigger_datetime}`}>
                    {record.project_name}/{record.job_name}/{record.task_name}
                </Link>
            </Descriptions.Item>*/}
            <Descriptions.Item label="Project">
                <Link to={`/projects/${record.project_id}`}>
                    {record.project_name}
                </Link>
            </Descriptions.Item>
            <Descriptions.Item label="Job">
                <Link to={`/jobs/${record.job_id}`}>
                    {record.job_name}
                </Link>
            </Descriptions.Item>
            <Descriptions.Item label="Attempt">
                {record.attempt}
            </Descriptions.Item>
            <Descriptions.Item label="Queued Time">
                <RelDate>{record.queued_datetime}</RelDate>
            </Descriptions.Item>
            <Descriptions.Item label="Start Time">
                <RelDate>{record.started_datetime}</RelDate>
            </Descriptions.Item>
            <Descriptions.Item label="Start Delay">
                `${record.queued_datetime}/${record.started_datetime}``
            </Descriptions.Item>
            <Descriptions.Item label="Finished Time">
                {record.finish_datetime &&
                    <RelDate>{record.finish_datetime}</RelDate>
                }
            </Descriptions.Item>
            <Descriptions.Item label="Running Duration">
                {record.finish_datetime &&
                    `${record.started_datetime}/${record.finish_datetime}`
                }
            </Descriptions.Item>
        </Descriptions>
    );
}


type WorkerProps = {};
type WorkerParams = {
    id: string;
};
type WorkerState = {
    worker?: WorkerType;
    filter: string[];
};

class Worker extends Component<WorkerProps, WorkerState> {
    columns: ColumnsType<WorkerTask>;

    constructor(props: WorkerProps) {
        super(props);

        this.columns = makeColumns();

        this.state = {
            filter: defaultFilter,
        }
    }

    async fetchWorker(id: string) {
        try {
            let resp = await axios.get<WorkerType>(`/api/workers/${id}`, {
                params: {
                    state: this.state.filter.join(',')
                }
            });
            this.setState({
                worker: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        const { id } = useParams() as WorkerParams;

        this.fetchWorker(id);
    }


    render() {
        const navigate = useNavigate();
        const { id } = useParams() as WorkerParams;
        const { worker } = this.state;

        const content = worker ? (
            <>
                <PageHeader
                    onBack={() => navigate(-1)}
                    title={`Worker ${id}`}
                    subTitle={
                        <Fragment>
                            <WorkerStatus status={worker.status} />
                            Version: {worker.version}
                        </Fragment>
                    }
                    extra={
                        <Button onClick={() => this.fetchWorker(id)}>
                            Refresh
                        </Button>
                    }
                />
                <Row gutter={[16, 32]}>
                    <Col span={6}>
                        <Statistic title="Running Tasks"
                            valueStyle={{color: geekblue[5]}}
                            value={worker.running_tasks} />
                    </Col>
                    <Col span={6}>
                        <Statistic title="Total Tasks"
                            valueStyle={{color: geekblue[5]}}
                            value={worker.total_tasks} />
                    </Col>
                    <Col span={6}>
                        <Statistic title="Last Seen"
                            value={worker.last_seen_datetime}
                            formatter={(val) => <RelDate>{dayjs(val).fromNow()}</RelDate>}
                            />
                    </Col>
                    <Col span={24} />
                </Row>
                <Select
                    mode="multiple"
                    defaultValue={defaultFilter}
                    style={{ width: 350 }}
                    onChange={(value) => {
                    this.setState({
                        filter: value
                    }, () => {
                        this.fetchWorker(id);    
                    });
                    }}
                >
                    <Option value="active">Active</Option>
                    <Option value="running">Running</Option>
                    <Option value="success">Success</Option>
                    <Option value="failure">Failure</Option>
                    <Option value="error">Error</Option>
                </Select>
                <Table key="1"
                    rowKey="task_run_id"
                    columns={this.columns}
                    dataSource={worker.tasks}
                    expandable={{
                        expandedRowRender: record => expandedRowRender(record),
                        expandRowByClick: true,
                        expandIcon: ({ expanded, onExpand, record }) =>
                            expanded ? (
                                <DownOutlined onClick={e => onExpand(record, e)} />
                            ) : (
                                <RightOutlined onClick={e => onExpand(record, e)} />
                            )
                    }}
                    />
            </>
        ) : <Spin size="large" />;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/workers">Workers</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/workers/${id}`}>{id}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>{content}</Body>
                </Content>
            </Layout>
        );
    }
}

export default Worker;
