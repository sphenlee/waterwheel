declare module 'react-graph-vis' {
    import { ReactElement } from "react";

    type GraphRep = {
        nodes: GraphNode[];
        edges: GraphEdge[];
    };

    type GraphNode = {  // TODO: This should be PartItem from vis-data/data-interface.ts
        id: string;
        label: string;
        title: string;
        shape: string;
        color: string;
    };

    type GraphEdge = {
        to: string;
        from: string;
    };

    type GraphOptions = {
        width: string;
        height: string;
        edges: {
            smooth: boolean;
        };
    };

    export default function Graph(props: {
        graph: GraphRep,
        options: GraphOptions,
    }): ReactElement;
}
