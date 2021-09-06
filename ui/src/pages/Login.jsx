import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Layout, Input, Button } from 'antd';
import { UserOutlined, LockOutlined } from '@ant-design/icons';

import axios from 'axios';

import Body from '../components/Body.jsx';
import Navigation from '../components/Navigation.jsx'

const { Content } = Layout;


class Login extends Component {
  render() {
    return (
      <Layout>
        <Content style={{padding: '50px'}}>
          <Body>
            <form action="/login" method="post">
              <Input
                name="username"
                prefix={<UserOutlined/>}
                placeholder="Username" />
              <Input
                name="password"
                prefix={<LockOutlined/>}
                type="password"
                placeholder="Password"
              />
              <Button type="primary" htmlType="submit">
                Log in
              </Button>
            </form>
          </Body>
        </Content>
      </Layout>
    );
  }
}

export default Login;

