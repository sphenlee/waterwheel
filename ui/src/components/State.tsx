import React, { Component } from "react";
import { Tag } from 'antd';
import {
  CheckCircleOutlined,
  SyncOutlined,
  CloseSquareOutlined,
  ExclamationCircleOutlined,
  ClockCircleOutlined,
  MinusCircleOutlined,
  WarningOutlined,
} from '@ant-design/icons';

type States = 'active' | 'running' | 'waiting' | 'success' | 'failure' | 'error';
type StateProps = {
  state: States
};

class State extends Component<StateProps> {
  render() {
    const { state } = this.props;

    let color;
    let icon;
    if (state == 'active') {
      color = 'default';
      icon = <SyncOutlined/>;
    } else if (state == 'running') {
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
      icon = <CloseSquareOutlined />;
    } else if (state == 'error') {
      color = 'warning';
      icon = <WarningOutlined />;
    } else {
      color = 'warning';
      icon = <WarningOutlined />;
    }


    return (
      <Tag icon={icon} color={color}>{state}</Tag>
    );
  }
}

export default State;

