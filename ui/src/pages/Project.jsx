import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { List, Avatar, Layout, Breadcrumb, PageHeader } from 'antd';
import axios from 'axios';

import Body from '../components/Body.jsx';

const { Content } = Layout;


class Project extends Component {
    constructor(props) {
        super(props);

        this.state = {
            loading: false,
            proj: {},
            jobs: []
        };
    }

    async fetchProject() {
        const {match} = this.props;
        
        try {
            this.setState({
                loading: true
            });
            let proj = await axios.get(`/api/projects/${match.params.id}`);
            let jobs = await axios.get(`/api/projects/${match.params.id}/jobs`);
            this.setState({
                loading: false,
                proj: proj.data,
                jobs: jobs.data,
            });
        } catch(e) {
            console.log(e);
            this.setState({
                loading: false,
                proj:{},
                jobs: []
            });
        }
    }

    componentDidMount() {
        this.fetchProject()
    }

    render() {
        const { history } = this.props;
        const { proj } = this.state;

        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to={`/projects/${proj.id}`}>{proj.name || "..."}</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <PageHeader
                            onBack={() => history.replace("/projects")}
                            title={proj.name}
                            subTitle={proj.description}
                        />
                        <List
                            itemLayout="vertical"
                            dataSource={this.state.jobs}
                            loading={this.state.loading}
                            renderItem={item => (
                                <List.Item>
                                    <List.Item.Meta
                                        avatar={<Avatar shape="square">{item.avatar}</Avatar>}
                                        title={<Link to={`/jobs/${item.job_id}`}>
                                            {item.name}
                                        </Link>}
                                        description={item.description}
                                    />
                                </List.Item>
                            )}
                        />
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Project;

