import React, { Component, Fragment } from "react";
import { Link } from "react-router-dom";
import Graph, { GraphRep } from "react-graph-vis";
import { Table, Select, notification, Spin } from 'antd';
import { geekblue, lime, red, grey, yellow } from '@ant-design/colors';
import axios from 'axios';

import { JobGraph as JobGraphType, JobGraphNode, JobGraphNodeState } from "../types/Job";

const options = {
    width: '100%',
    height: '500px',
    edges: {
        smooth: true,
    },
    /* TODO - in theory we want this but it looks terrible with the default options
    layout: {
        hierarchical: {
            enabled: true,
            sortMethod: 'directed',
        }
    }*/
}

function stateColor(state: JobGraphNodeState | null) {
    return state ? {
        waiting: grey[3],
        active: geekblue[3],
        success: lime[3],
        failure: red[3],
    }[state] : grey[0];
}


type JobGraphProps = {
    id: string;
    trigger_datetime?: string;
};

type JobGraphState = {
    loading: boolean;
    graph: GraphRep | null;
};

class JobGraph extends Component<JobGraphProps, JobGraphState> {
    interval: NodeJS.Timeout;

    constructor(props: JobGraphProps) {
        super(props);

        this.state = {
            loading: false,
            graph: null
        }
    }

    createGraph(data: JobGraphType, id: string): GraphRep {
        const nodeLabel = (n: JobGraphNode) => {
            if (n.job_id === id) {
                return `${n.name}`;
            } else {
                return `(${n.name})`
            }
        }

        const nodeTitle = (n: JobGraphNode) => {
            if (n.job_id === id) {
                return `task ${n.name}`;
            } else {
                return `task ${n.name} from job <a href="/">${n.job_id}</a>`
            }   
        }

        return {
            nodes: data.nodes.map(n => ({
                id: n.id,
                label: nodeLabel(n),
                title: nodeTitle(n),
                shape: 'box',
                color: (n.kind === 'trigger' ? yellow[3] : stateColor(n.state))
            })),
            edges: data.edges.map(e => ({
                to: e.to,
                "from": e.from,
                arrows: {
                    middle: {
                        enabled: (e.kind == 'failure'),
                        scaleFactor: 0.5,
                        type: 'bar',
                    }
                }
            })),
          };
    }

    async fetchGraph() {
        const { id, trigger_datetime } = this.props;

        if (id === undefined) {
            return;
        }

        try {
            this.setState({
                loading: true
            });

            let url;
            if (trigger_datetime) {
                url = `/api/jobs/${id}/graph?trigger_datetime=${trigger_datetime}`;
            } else {
                url = `/api/jobs/${id}/graph`;
            }

            let resp = await axios.get<JobGraphType>(url);

            this.setState({
                graph: this.createGraph(resp.data, id),
                loading: false,
            });
        } catch(e) {
            this.setState({
                loading: false,
            });

            notification.error({
                message: 'Error fetching Job Graph',
                description: e,
                placement: 'bottomLeft',
            });
        }
    }

    componentDidMount() {
        this.fetchGraph()

        if (this.props.trigger_datetime) {
            this.interval = setInterval(() => this.fetchGraph(), 5000);
        }
    }

    componentDidUpdate(oldprops: JobGraphProps) {
        if (this.props.id != oldprops.id) {
            this.fetchGraph()
        }
    }

    componentWillUnmount() {
        if (this.interval) {
            clearInterval(this.interval);
        }
    }

    render() {
        const { id } = this.props;
        const { graph, loading } = this.state;

        return (
            <Spin spinning={loading} size="large" tip="Loading..." delay={200}>
                { graph && <Graph graph={graph} options={options}/> }
            </Spin>
        );
    }
}

export default JobGraph;
