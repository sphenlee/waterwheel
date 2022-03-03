import React, { Component, Fragment } from "react";
import { Tag } from 'antd';
import {
  CheckOutlined,
  PoweroffOutlined,
  WarningOutlined,
} from '@ant-design/icons';

type WorkerStatusProps = {
  status: string;
};

function WorkerStatus({status}: WorkerStatusProps) {
    let color;
    let icon;
    if (status == 'up') {
      color = 'success';
      icon = <CheckOutlined/>;
    } else if (status == 'gone') {
      color = 'warning';
      icon = <PoweroffOutlined/>;
    } else {
      color = 'error';
      icon = <WarningOutlined />;
      status = 'error';
    }

    return (
      <Tag icon={icon} color={color}>{status}</Tag>
    );
}

export default WorkerStatus;
