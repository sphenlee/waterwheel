import React, { Component } from "react";
import { Tag } from 'antd';
import {
  CheckCircleOutlined,
  SyncOutlined,
  CloseCircleOutlined,
  ExclamationCircleOutlined,
  ClockCircleOutlined,
  MinusCircleOutlined,
} from '@ant-design/icons';


class State extends Component {
  render() {
    const { state } = this.props;

    let color;
    let icon;
    if (state == 'active') {
      color = 'processing';
      icon = <SyncOutlined spin/>;
    } else if (state == 'waiting') {
      color = 'default';
      icon = <ClockCircleOutlined />;
    } else if (state == 'success') {
      color = 'success';
      icon = <CheckCircleOutlined />;
    } else if (state == 'failure') {
      color = 'error';
      icon = <CloseCircleOutlined />;
    } else {

    }


    return (
      <Tag icon={icon} color={color}>{state}</Tag>
    );
  }
}

export default State;

