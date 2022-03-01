import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { List, Avatar, Layout, Breadcrumb, Row, Col } from 'antd';
import { ProjectOutlined } from '@ant-design/icons';
import axios from 'axios';

import Body from '../components/Body';

const { Content } = Layout;

type ProjectsState = {
    loading: boolean;
    data: any[];
};


class Projects extends Component<{}, ProjectsState> {
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
                        <Row>
                            <Col span={12}>
                                <List
                                    itemLayout="vertical"
                                    bordered={true}
                                    dataSource={this.state.data}
                                    loading={this.state.loading}
                                    renderItem={(item: any) => (
                                        <List.Item>
                                            <List.Item.Meta
                                                avatar={<Avatar icon={<ProjectOutlined />} shape="square"></Avatar>}
                                                title={<Link to={`/projects/${item.id}`}>
                                                    {item.name}
                                                </Link>}
                                                description={item.description}
                                            />
                                        </List.Item>
                                    )}
                                />
                            </Col>
                        </Row>
                    </Body>
                </Content>
            </Layout>
        );
    }
}

export default Projects;

