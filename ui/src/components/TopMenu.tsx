import React, { Component } from "react";
import { Link, withRouter } from 'react-router-dom';
import { Layout, Menu } from 'antd';

class TopMenu extends Component {
  render() {
    return (
      <Layout.Header className="header">
        <Menu theme="dark" mode="horizontal">
          <Menu.Item key="home">
            <Link to="/">
              Home
            </Link>
          </Menu.Item>
          <Menu.Item key="projects">
            <Link to="/projects">
              Projects
            </Link>
          </Menu.Item>
          <Menu.Item key="workers">
            <Link to="/workers">
              Workers
            </Link>
          </Menu.Item>
        </Menu>
      </Layout.Header>
    );
  }
}

export default withRouter(TopMenu);
