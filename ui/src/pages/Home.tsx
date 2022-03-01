import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import { Layout, Breadcrumb, Row, Col, Statistic } from 'antd';
import { geekblue, lime, red, grey, yellow } from '@ant-design/colors';
import axios from 'axios';

import Body from '../components/Body';
import Navigation from '../components/Navigation.jsx'

const { Content } = Layout;

type HomeState = {
  loading: boolean;
  status: {
    num_projects?: number;
    num_workers?: number;
    running_tasks?: number;
  };
};


class Home extends Component<{}, HomeState> {
  interval: NodeJS.Timeout;

  constructor(props) {
      super(props);

      this.state = {
          loading: true,
          status: {},
      };
  }

  async fetchStatus() {
      try {
          this.setState({
              loading: true,
          });
          let resp = await axios.get(`/api/status`);
          this.setState({
              status: resp.data,
              loading: false,
          });
      } catch(e) {
          console.log(e);
      }
  }

  componentDidMount() {
      this.fetchStatus()

      this.interval = setInterval(() => this.fetchStatus(), 5000);
  }

  componentWillUnmount() {
      clearInterval(this.interval);
  }

  render() {
    const { status } = this.state;

    return (
      <Layout>
        <Content style={{padding: '50px'}}>
          <Breadcrumb style={{paddingBottom: '12px'}}>
              <Breadcrumb.Item><Link to="/">Home</Link></Breadcrumb.Item>
          </Breadcrumb>
          <Body>
            <Row gutter={[16, 32]}>
                <Col span={6}>
                    <Statistic title="Projects"
                        valueStyle={{color: geekblue[5]}}
                        value={status.num_projects} />
                </Col>
                <Col span={6}>
                    <Statistic title="Workers"
                        valueStyle={{color: geekblue[5]}}
                        value={status.num_workers} />
                </Col>
                <Col span={6}>
                    <Statistic title="Running Tasks"
                        valueStyle={{color: geekblue[5]}}
                        value={status.running_tasks} />
                </Col>
            </Row>
          </Body>
        </Content>
      </Layout>
    );
  }
}

export default Home;

