import React, { Component } from "react";
import { Button, notification } from 'antd';

import axios from 'axios';


class ActivateToken extends Component {
    constructor(props) {
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
        })
    }

    render() {
        const { loading } = this.state;
        return (
            <Button
                size="small"
                loading={loading}
                onClick={() => this.createToken()}
            >activate</Button>
        );
    }
}

export default ActivateToken;
