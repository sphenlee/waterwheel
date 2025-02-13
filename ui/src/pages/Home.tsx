import React, { Component } from "react";
import { Link } from "react-router-dom";
import { Layout, Breadcrumb, Row, Col, Statistic } from 'antd';
import { geekblue, lime, red, grey, yellow } from '@ant-design/colors';
import axios from 'axios';

import Body from '../components/Body';
import { Status } from "../types/Status";
import { interval } from "../types/common";

const { Content } = Layout;

type HomeState = {
  status?: Status;
};


class Home extends Component<{}, HomeState> {
  interval: interval;

  constructor(props: {}) {
      super(props);

      this.state = {};
  }

  async fetchStatus() {
      try {
          let resp = await axios.get<Status>(`/api/status`);
          this.setState({status: resp.data});
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
                        value={status?.num_projects ?? 0} />
                </Col>
                <Col span={6}>
                    <Statistic title="Workers"
                        valueStyle={{color: geekblue[5]}}
                        value={status?.num_workers ?? 0} />
                </Col>
                <Col span={6}>
                    <Statistic title="Running Tasks"
                        valueStyle={{color: geekblue[5]}}
                        value={status?.running_tasks ?? 0} />
                </Col>
            </Row>
          </Body>
        </Content>
      </Layout>
    );
  }
}

export default Home;

