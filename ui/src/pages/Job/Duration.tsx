import React, { Component, Fragment } from "react";
import { Row, Col, Button, DatePicker, Space } from 'antd';
import axios from 'axios';
import { Line } from '@ant-design/charts';

import {
  DoubleRightOutlined,
} from '@ant-design/icons';

const config = {
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
    data: any | null;
    limit: number;
    before: any | null;
};

class Duration extends Component<DurationProps, DurationState> {
    constructor(props) {
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

    onDatePicked(date) {
        this.fetchDuration(date.toISOString());
    }

    async fetchDuration(before) {
        const { id } = this.props;
        const { limit } = this.state;

        let params = {
            limit: limit,
            before: before,
        };

        let resp = await axios.get(`/api/jobs/${id}/duration`, {
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
                        <Line loading={data === null} data={data?.duration ?? []} {...config} />
                    </Col>
                </Row>
            </Fragment>
        );
    }
}

export default Duration;
