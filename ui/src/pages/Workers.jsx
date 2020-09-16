import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Table, Layout, Breadcrumb } from 'antd';
import axios from 'axios';
import Moment from 'react-moment';

import Body from '../components/Body.jsx';

const { Content } = Layout;


function makeColumns() {
    return [
        {
            title: 'Id',
            dataIndex: 'uuid',
            render: (text, record) => (
                <Link to={`/workers/${record.uuid}`}>
                    {text}
                </Link>
            ),
        },{
            title: 'Running Tasks',
            dataIndex: 'running_tasks',
        },{
            title: 'Total Tasks',
            dataIndex: 'total_tasks',
        },{
            title: 'UI Address',
            dataIndex: 'addr',
            render: text => <a href={`http://${text}`}>{text}</a>,
        },{
            title: 'Last Seen',
            dataIndex: 'last_seen_datetime',
            render: text => <Moment fromNow withTitle>{text}</Moment>
        }
    ];
}


class Workers extends Component {
    constructor(props) {
        super(props);

        this.columns = makeColumns();

        this.state = {
            loading: false,
            workers: []
        };
    }

    async fetchWorkers() {
        try {
            this.setState({
                loading: true
            });
            let resp = await axios.get('/api/workers');
            this.setState({
                loading: false,
                workers: resp.data
            });
        } catch(e) {
            console.log(e);
            this.setState({
                loading: false,
                workers:[]
            });
        }
    }

    componentDidMount() {
        this.fetchWorkers()
        this.interval = setInterval(() => this.fetchWorkers(), 5000);
    }

    componentWillUnmount() {
        clearInterval(this.interval);
    }

    render() {
        const { workers, loading } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/workers">Workers</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <Table columns={this.columns} dataSource={workers} loading={loading}/>
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Workers;

