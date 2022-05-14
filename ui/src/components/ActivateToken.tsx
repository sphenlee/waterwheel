import React, { Component } from "react";
import { Button, notification } from 'antd';

import axios from 'axios';
import { ButtonType } from "antd/lib/button";
import { SizeType } from "antd/lib/config-provider/SizeContext";

type ActivateTokenProps = {
    task_id: string;
    trigger_datetime: string;
    type: ButtonType,
    size?: SizeType,
};

type ActivateTokenState = {
    loading: boolean;
};


class ActivateToken extends Component<ActivateTokenProps, ActivateTokenState> {
    constructor(props: ActivateTokenProps) {
        super(props);
        this.state = {
            loading: false
        };
    }

    async createToken() {
        const { task_id, trigger_datetime } = this.props;
        this.setState({ loading: true });
        await axios.put(`/api/tasks/${task_id}/tokens/${trigger_datetime}`, {});
        this.setState({ loading: false });
        notification.success({
            message: 'Task Activated',
            description: 'The task has been activated and will run shortly.',
            placement: 'bottomLeft',
        });

    }

    render() {
        const { loading } = this.state;
        const { type, size } = this.props;
        return (
            <Button
                loading={loading}
                type={type}
                size={size}
                onClick={() => this.createToken()}
            >activate</Button>
        );
    }
}

export default ActivateToken;
