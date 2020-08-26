import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Layout, Breadcrumb } from 'antd';
import styled from 'styled-components';

import Navigation from '../components/Navigation.jsx'

const { Content } = Layout;

const Body = styled.div`
    padding: 24px;
    background: #fff;
`;

class Home extends Component {
  render() {
    return (
      <Layout>
        <Content style={{padding: '50px'}}>
          <Breadcrumb style={{paddingBottom: '12px'}}>
              <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
          </Breadcrumb>
          <Body>
            Overview of system status here... TODO
          </Body>
        </Content>
      </Layout>
    );
  }
}

export default Home;

