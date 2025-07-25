import React, { Component } from "react";
import { Link } from 'react-router-dom';
import { Layout, Menu } from 'antd';

class TopMenu extends Component {
  render() {
    return (
      <Layout.Header className="header">
        <Menu
            theme="dark"
            mode="horizontal"
            items={[
                {
                    key: "home",
                    label: <Link to="/">Home</Link>
                },{
                    key: "projects",
                    label: <Link to="/projects">Projects</Link>
                },{
                    key: "schedulers",
                    label: <Link to="/schedulers">Schedulers</Link>
                },{
                    key: "workers",
                    label: <Link to="/workers">Workers</Link>
                }
            ]}
        />
      </Layout.Header>
    );
  }
}

export default TopMenu;
