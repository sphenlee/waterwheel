import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Layout, Breadcrumb } from 'antd';

import Body from '../components/Body.jsx';
import Navigation from '../components/Navigation.jsx'

const { Content } = Layout;


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

