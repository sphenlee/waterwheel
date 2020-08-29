import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { List, Avatar, Layout, Breadcrumb } from 'antd';
import axios from 'axios';

import Body from '../components/Body.jsx';

const { Content } = Layout;


class Projects extends Component {
    constructor(props) {
        super(props);

        this.state = {
            loading: false,
            data: []
        };
    }

    async fetchProjects() {
        try {
            this.setState({
                loading: true
            });
            let resp = await axios.get('/api/projects');
            this.setState({
                loading: false,
                data: resp.data
            });
        } catch(e) {
            console.log(e);
            this.setState({
                loading: false,
                data:[]
            });
        }
    }

    componentDidMount() {
        this.fetchProjects()
    }

    render() {
        return (
            <Layout>
                <Content style={{padding: '50px'}}>
                    <Breadcrumb style={{paddingBottom: '12px'}}>
                        <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
                        <Breadcrumb.Item><Link to="/projects">Projects</Link></Breadcrumb.Item>
                    </Breadcrumb>
                    <Body>
                        <List
                            itemLayout="vertical"
                            dataSource={this.state.data}
                            loading={this.state.loading}
                            renderItem={item => (
                                <List.Item>
                                    <List.Item.Meta
                                        avatar={<Avatar shape="square">{item.avatar}</Avatar>}
                                        title={<Link to={`/projects/${item.id}`}>
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

export default Projects;

