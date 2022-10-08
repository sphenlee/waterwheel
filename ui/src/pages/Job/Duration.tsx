import React, { Component, Fragment } from "react";
import { Row, Col, Button, DatePicker, Space } from 'antd';
import axios from 'axios';
import { Column } from '@ant-design/charts';

import {
  DoubleRightOutlined,
} from '@ant-design/icons';
import { TaskDuration, TaskDurationList } from "../../types/Task";
import { Moment } from "moment";

const config = {
    isStack: true,
    xField: 'trigger_datetime',
    yField: 'duration',
    seriesField: 'task_name',
    yAxis: {
        title: {
            text: 'Task Duration (s)'
        }
    },
    xAxis: {
        title: {
            text: 'Trigger Date'
        }
    }
  };

type DurationProps = {
    id: string;
};
type DurationState = {
    data: TaskDurationList | null;
    limit: number;
    before: string | null;  // TODO: datetime
};

class Duration extends Component<DurationProps, DurationState> {
    constructor(props: DurationProps) {
        super(props);

        this.state = {
            data: null,
            limit: 25,
            before: null,
        }
    }

    gotoCurrent() {
        this.fetchDuration(null);
    }

    onDatePicked(date: Moment | null) {
        if(date) {
            this.fetchDuration(date.toISOString());
        }
    }

    async fetchDuration(before: string | null) {
        const { id } = this.props;
        const { limit } = this.state;

        let params = {
            limit: limit,
            before: before,
        };

        let resp = await axios.get<TaskDurationList>(`/api/jobs/${id}/duration`, {
            params: params
        });

        this.setState({
            data: resp.data,
        });
    }

    componentDidMount() {
        this.gotoCurrent();
    }

    render() {
        const { id }  = this.props;
        const { data } = this.state;

        return (
            <Fragment>
                <Row>
                    <DatePicker onChange={(date) => this.onDatePicked(date)} />
                    <Space />
                    <Button onClick={() => this.gotoCurrent()} icon={<DoubleRightOutlined />}>
                        Latest
                    </Button>
                </Row>
                <Row>
                    <Col span={24}>
                        <Column loading={data === null} data={data?.duration ?? []} {...config} />
                    </Col>
                </Row>
            </Fragment>
        );
    }
}

export default Duration;
