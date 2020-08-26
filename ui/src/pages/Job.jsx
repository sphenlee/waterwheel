import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { List, Avatar, Layout, Breadcrumb, PageHeader, Collapse, Tabs } from 'antd';
import JSONPretty from 'react-json-pretty';
import styled from 'styled-components';
import axios from 'axios';

import TokenTable from './Job/TokenTable.jsx';
import Triggers from './Job/Triggers.jsx';

const { Content } = Layout;


const Body = styled.div`
    padding: 24px;
    background: #fff;
`;

class Job extends Component {
    constructor(props) {
        super(props);

        this.state = {
            job: {},
            tokens: []
        };
    }

    async fetchJob() {
        const {match} = this.props;
        
        try {
            let resp = await axios.get(`/api/jobs/${match.params.id}`);
            this.setState({
                job: resp.data,
            });
        } catch(e) {
            console.log(e);
        }
    }

    componentDidMount() {
        this.fetchJob();
    }

    render() {
        const { history } = this.props;
        const { job } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/projects/${job.project_id}`}>{job.project || "..."}</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/jobs/${job.id}`}>{job.name || "..."}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <PageHeader
                            onBack={() => history.replace(`/projects/${job.project_id}`)}
                            title={job.name}
                            subTitle={job.description}
                        />
                        <Tabs>
                            <Tabs.TabPane tab="Overview" key="1">
                              TODO
                            </Tabs.TabPane>
                            <Tabs.TabPane tab="Triggers" key="2">
                                <Triggers id={job.id} job={job}/>
                            </Tabs.TabPane>
                            <Tabs.TabPane tab="Tokens" key="3">
                                <TokenTable id={job.id}/>
                            </Tabs.TabPane>
                            <Tabs.TabPane tab="Definition" key="4">
                                <JSONPretty data={job.raw_definition} />
                            </Tabs.TabPane>
                        </Tabs>
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Job;

