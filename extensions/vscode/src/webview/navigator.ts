// Browser-side script for the LID Intent Navigator WebView.
// Bundled by esbuild into media/navigator.js — runs in the webview sandbox,
// not the extension host. No Node.js APIs available.

import cytoscape, { ElementDefinition, NodeSingular } from 'cytoscape';
// @ts-ignore — cytoscape-dagre ships no bundled types
import dagreLayout from 'cytoscape-dagre';

cytoscape.use(dagreLayout);

// acquireVsCodeApi is injected by VS Code into every webview context.
declare function acquireVsCodeApi(): { postMessage(data: unknown): void };
const vscode = acquireVsCodeApi();

// ── Types shared with the extension host ────────────────────────────────────

interface GraphNode {
    id: string;
    label: string;
    status: string;
}

interface GraphEdge {
    source: string;
    target: string;
}

interface GraphPayload {
    nodes: GraphNode[];
    edges: GraphEdge[];
}

// ── Status colour palette (mirrors LID arrow status enum) ───────────────────

const STATUS_COLOR: Record<string, string> = {
    UNMAPPED: '#6b7280', // grey
    MAPPED:   '#3b82f6', // blue
    AUDITED:  '#f59e0b', // amber
    OK:       '#22c55e', // green
    MERGED:   '#9ca3af', // muted grey
};

function colorForStatus(status: string): string {
    return STATUS_COLOR[status] ?? STATUS_COLOR['UNMAPPED']!;
}

// ── Cytoscape instance ───────────────────────────────────────────────────────

let cy: cytoscape.Core | undefined;

const LAYOUT_OPTIONS = {
    name: 'dagre',
    rankDir: 'TB',
    ranker: 'network-simplex',
    nodeSep: 40,
    rankSep: 60,
    animate: false,
};

function render(payload: GraphPayload): void {
    const elements: ElementDefinition[] = [
        ...payload.nodes.map((n) => ({
            data: { id: n.id, label: n.label, status: n.status },
        })),
        ...payload.edges.map((e, i) => ({
            data: { id: `e${i}`, source: e.source, target: e.target },
        })),
    ];

    const emptyDiv = document.getElementById('empty');
    if (emptyDiv) {
        emptyDiv.style.display = payload.nodes.length === 0 ? 'block' : 'none';
    }

    if (cy) {
        cy.elements().remove();
        if (elements.length > 0) {
            cy.add(elements);
            cy.layout(LAYOUT_OPTIONS as Parameters<cytoscape.Core['layout']>[0]).run();
            cy.fit();
        }
        return;
    }

    cy = cytoscape({
        container: document.getElementById('cy'),
        elements,
        style: [
            {
                selector: 'node',
                style: {
                    label: 'data(label)',
                    'background-color': (ele: NodeSingular) =>
                        colorForStatus(ele.data('status') as string),
                    color: '#ffffff',
                    'font-size': '11px',
                    'font-family': 'var(--vscode-font-family, monospace)',
                    'text-valign': 'center',
                    'text-halign': 'center',
                    'text-wrap': 'wrap',
                    'text-max-width': '110px',
                    width: 130,
                    height: 44,
                    shape: 'round-rectangle',
                    'border-width': 0,
                },
            },
            {
                selector: 'node:selected',
                style: {
                    'border-width': 3,
                    'border-color': '#ffffff',
                },
            },
            {
                selector: 'node:active',
                style: { 'overlay-opacity': 0.1 },
            },
            {
                selector: 'edge',
                style: {
                    width: 1.5,
                    'line-color': '#6b7280',
                    'target-arrow-color': '#6b7280',
                    'target-arrow-shape': 'triangle',
                    'curve-style': 'bezier',
                    'arrow-scale': 0.8,
                },
            },
            {
                selector: 'edge:selected',
                style: {
                    'line-color': '#ffffff',
                    'target-arrow-color': '#ffffff',
                },
            },
        ],
        layout: LAYOUT_OPTIONS as Parameters<cytoscape.Core['layout']>[0],
        minZoom: 0.2,
        maxZoom: 3,
    });

    cy.on('tap', 'node', (event) => {
        const id = event.target.id() as string;
        vscode.postMessage({ type: 'open', segmentId: id });
    });
}

// ── Message bridge ───────────────────────────────────────────────────────────

window.addEventListener(
    'message',
    (event: MessageEvent<{ type: string; payload: GraphPayload }>) => {
        if (event.data.type === 'graph') {
            render(event.data.payload);
        }
    },
);
