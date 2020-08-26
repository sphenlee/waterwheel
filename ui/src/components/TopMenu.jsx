import React, { Component } from "react";
import { Link } from 'react-router-dom';
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
          <Menu.Item key="settings">Settings</Menu.Item>
          <Menu.Item key="admin">Admin</Menu.Item>
        </Menu>
      </Layout.Header>
    );
  }
}

export default TopMenu;

