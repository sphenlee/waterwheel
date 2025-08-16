import React, { Component, Fragment, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { List, Avatar, Layout, Breadcrumb, Row, Col } from 'antd';
import { ProjectOutlined } from '@ant-design/icons';
import axios from 'axios';

import Body from '../components/Body';
import { Project as ProjectType } from "../types/Project";

const { Content } = Layout;

type ProjectsState = {
    loading: boolean;
    data: ProjectType[];
};


function Projects() {
    const [loading, setLoading] = useState(false);
    const [data, setData] = useState([] as ProjectType[]);

    async function fetchProjects() {
        try {
            setLoading(true);
            let resp = await axios.get<ProjectType[]>('/api/projects');
            setData(resp.data);
            setLoading(false);
        } catch(e) {
            console.log(e);
            setData([]);
            setLoading(false);
        }
    }

    useEffect(() => {
        fetchProjects();
    }, []);
    
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
                                dataSource={data}
                                loading={loading}
                                renderItem={(item: ProjectType) => (
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

export default Projects;

