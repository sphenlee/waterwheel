import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb, PageHeader, Row, Col, Statistic,
    Descriptions, Button, Select } from 'antd';
import { geekblue } from '@ant-design/colors';
import { RightOutlined, DownOutlined } from "@ant-design/icons";


const { Option } = Select;

import axios from 'axios';
import Moment from 'react-moment';

import Body from '../components/Body';
import State from '../components/State';
import RelDate from '../components/Date';
import WorkerStatus from '../components/WorkerStatus';
import { ColumnsType } from "antd/lib/table";

const { Content } = Layout;

const defaultFilter = ["active", "running"];

function makeColumns() {
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


function expandedRowRender(record) {
    return (
        <Descriptions
                size="small"
                bordered
                column={2}
                labelStyle={{
                    fontWeight: "bold"
                }}
                contentStyle={{
                    background: "#fff"
                }}>
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
            <Descriptions.Item label="Queued Time" span={2}>
                <RelDate>{record.queued_datetime}</RelDate>
            </Descriptions.Item>
            <Descriptions.Item label="Start Time">
                <RelDate>{record.started_datetime}</RelDate>
            </Descriptions.Item>
            <Descriptions.Item label="Start Delay">
                <Moment duration={record.queued_datetime} date={record.started_datetime} />
            </Descriptions.Item>
            <Descriptions.Item label="Finished Time">
                {record.finish_datetime &&
                    <RelDate>{record.finish_datetime}</RelDate>
                }
            </Descriptions.Item>
            <Descriptions.Item label="Running Duration">
                {record.finish_datetime &&
                    <Moment duration={record.started_datetime} date={record.finish_datetime} />
                }
            </Descriptions.Item>
        </Descriptions>
    );
}


type WorkerProps = {
    history: any;
    match: any;
};
type WorkerState = {
    tasks: any[];
    filter: any;
    last_seen_datetime?: any;
    running_tasks?: any;
    total_tasks?: any;
    version?: any;
    status?: any;
};

class Worker extends Component<WorkerProps, WorkerState> {
    columns: ColumnsType<any>;

    constructor(props: WorkerProps) {
        super(props);

        this.columns = makeColumns();

        this.state = {
            tasks: [],
            filter: defaultFilter,
        }
    }

    async fetchWorker(id) {
        try {
            let resp = await axios.get(`/api/workers/${id}`, {
                params: {
                    state: this.state.filter.join(',')
                }
            });
            this.setState({
                tasks: resp.data.tasks,
                last_seen_datetime: resp.data.last_seen_datetime,
                running_tasks: resp.data.running_tasks,
                total_tasks: resp.data.total_tasks,
                version: resp.data.version,
                status: resp.data.status,
            });
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        const { id } = this.props.match.params;

        this.fetchWorker(id);
    }


    render() {
        const { history, match } = this.props;
        const { id } = match.params;
        const { tasks, last_seen_datetime, running_tasks, total_tasks, version, status } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/workers">Workers</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/workers/${id}`}>{id}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <PageHeader
                            onBack={() => history.goBack()}
                            title={`Worker ${id}`}
                            subTitle={
                                <Fragment>
                                    <WorkerStatus status={status} />
                                    Version: {version}
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
                                    value={running_tasks} />
                            </Col>
                            <Col span={6}>
                                <Statistic title="Total Tasks"
                                    valueStyle={{color: geekblue[5]}}
                                    value={total_tasks} />
                            </Col>
                            <Col span={6}>
                                <Statistic title="Last Seen"
                                    value={last_seen_datetime}
                                    formatter={(val) => <Moment fromNow withTitle>{val}</Moment>}
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
                            dataSource={tasks}
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
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Worker;
