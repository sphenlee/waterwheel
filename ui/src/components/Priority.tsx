import React, { Component } from "react";
import { Tag } from 'antd';
import {
    CaretLeftOutlined,
    BackwardOutlined,
    CaretRightOutlined,
    ForwardOutlined,
    WarningOutlined,
} from '@ant-design/icons';

type Priorities = 'backfill' | 'low' | 'normal' | 'high';
type PriorityProps = {
  priority: Priorities
};

class Priority extends Component<PriorityProps> {
  render() {
    const { priority } = this.props;

    let color;
    let icon;
    if (priority == 'backfill') {
      icon = <BackwardOutlined />;
    } else if (priority == 'low') {
      icon = <CaretLeftOutlined />;
    } else if (priority == 'normal') {
      icon = <CaretRightOutlined />;
    } else if (priority == 'high') {
      icon = <ForwardOutlined />;
    } else {
      icon = <WarningOutlined />;
    }


    return (
      <Tag icon={icon} color={'default'}>{priority}</Tag>
    );
  }
}

export default Priority;

