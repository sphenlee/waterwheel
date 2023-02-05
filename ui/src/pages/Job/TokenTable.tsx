import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Select, notification } from 'antd';
import axios from 'axios';

import State from '../../components/State';
import { ColumnsType } from "antd/lib/table";
import { Token } from "../../types/Token";

const { Option } = Select;

function makeColumns(job_id: string): ColumnsType<Token> {
    return [
      {
        title: 'Trigger Time',
        dataIndex: 'trigger_datetime',
        key: 'trigger_datetime',
        render: (text, record) => (
                <Link to={`/jobs/${job_id}/tokens/${record.trigger_datetime}`}>
                    {text}
                </Link>
            )
      },{
        title: 'Task',
        dataIndex: 'task_name',
        key: 'task_name',
        render: text => text,
      },{
        title: 'State',
        dataIndex: 'state',
        key: 'state',
        render: text => <State state={text} />,
      }
    ];
}

type Filter = 'active' | 'running';
type TokenTableProps = {
    id: string;
};
type TokenTableState = {
    filter: Filter[];
    tokens: Token[];
};

const defaultFilter = ['running'];

class TokenTable extends Component<TokenTableProps, TokenTableState> {
    columns: ColumnsType<Token>;
    interval: NodeJS.Timeout;

    constructor(props: TokenTableProps) {
        super(props);

        this.columns = makeColumns(props.id);

        this.state = {
            filter: defaultFilter,
            tokens: [],
        }
    }

    async fetchTokens(id: string) {
        try {
            let resp = await axios.get<Token[]>(`/api/jobs/${id}/tokens`, {
                params: {
                    state: this.state.filter.join(',')
                }
            });
            this.setState({
                tokens: resp.data,
            });
        } catch(e) {
            notification.error({
                message: 'Error fetching Tokens',
                description: e,
                placement: 'bottomLeft',
            });
        }
    }

    componentDidMount() {
        const { id } = this.props;

        this.fetchTokens(id)

        this.interval = setInterval(() => this.fetchTokens(id), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { id } = this.props;
        const { tokens } = this.state;

        return (
            <Fragment>
                <Select
                  mode="multiple"
                  defaultValue={defaultFilter}
                  style={{ width: 350 }}
                  onChange={(value: Filter[]) => {
                    this.setState({
                        filter: value
                    }, () => {
                        this.fetchTokens(id);    
                    });
                  }}
                >
                    <Option value="active">Active</Option>
                    <Option value="running">Running</Option>
                    <Option value="success">Success</Option>
                    <Option value="failure">Failure</Option>
                    <Option value="waiting">Waiting</Option>
                    <Option value="error">Error</Option>
                </Select>

                <Table rowKey={record => record.trigger_datetime + record.task_name}
                    columns={this.columns}
                    dataSource={tokens}
                    loading={tokens === null}
                    pagination={{position: ['bottomLeft']}}
                    />
            </Fragment>
        );
    }
}

export default TokenTable;
