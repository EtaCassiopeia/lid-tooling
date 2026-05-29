// Browser-side script for the LID Intent Navigator WebView.
// Bundled by esbuild into media/navigator.js — runs in the webview sandbox,
// not the extension host. No Node.js APIs available.

import cytoscape, { ElementDefinition, NodeSingular } from 'cytoscape';
// @ts-ignore — cytoscape-dagre ships no bundled types
import dagreLayout from 'cytoscape-dagre';

cytoscape.use(dagreLayout);

declare function acquireVsCodeApi(): { postMessage(data: unknown): void };
const vscode = acquireVsCodeApi();

// ── Types (must stay in sync with src/navigator.ts) ─────────────────────────

interface SpecCounts {
    implemented: number;
    open: number;
    deferred: number;
}

interface GraphNode {
    id: string;
    label: string;
    status: string;
    next?: string;
    drift?: string;
    sampled?: string;
    audited?: string;
    specs?: SpecCounts;
    specFile?: string;
    lldFile?: string;
}

interface GraphEdge {
    source: string;
    target: string;
}

interface GraphPayload {
    nodes: GraphNode[];
    edges: GraphEdge[];
}

// ── Status colours ───────────────────────────────────────────────────────────

const STATUS_COLOR: Record<string, string> = {
    UNMAPPED: '#6b7280',
    MAPPED:   '#3b82f6',
    AUDITED:  '#f59e0b',
    OK:       '#22c55e',
    MERGED:   '#9ca3af',
};

function colorForStatus(status: string): string {
    return STATUS_COLOR[status] ?? STATUS_COLOR['UNMAPPED']!;
}

// ── Tiny utilities ───────────────────────────────────────────────────────────

function esc(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function trunc(s: string, n: number): string {
    return s.length <= n ? s : s.slice(0, n) + '…';
}

// Strip ISO time suffix: "2026-04-25T00:00:00.000Z" → "2026-04-25"
function fmtDate(s: string | undefined): string {
    if (!s) return '—';
    return s.replace(/T.*$/, '');
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

function buildLabel(n: GraphNode): string {
    const s = n.specs;
    if (!s) return n.id;
    const total = s.implemented + s.open + s.deferred;
    return total > 0 ? `${n.id}\n${s.implemented}/${total}` : n.id;
}

function render(payload: GraphPayload): void {
    const elements: ElementDefinition[] = [
        ...payload.nodes.map((n) => ({
            data: {
                id: n.id,
                label: buildLabel(n),
                status: n.status,
                hasDrift: n.drift ? 1 : 0,
                hasNext: n.next ? 1 : 0,
                nodeData: n,
            },
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
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        style: [
            {
                selector: 'node',
                style: {
                    'label': 'data(label)',
                    'background-color': (ele: NodeSingular) =>
                        colorForStatus(ele.data('status') as string),
                    'color': '#ffffff',
                    'font-size': '10px',
                    'font-family': 'var(--vscode-font-family, monospace)',
                    'text-valign': 'center',
                    'text-halign': 'center',
                    'text-wrap': 'wrap',
                    'text-max-width': '115px',
                    'width': 135,
                    'height': 54,
                    'shape': 'round-rectangle',
                    'border-width': 0,
                },
            },
            // Colored border indicators: amber = has drift, blue = has next.
            // Drift overrides next (defined later → higher specificity in Cytoscape).
            {
                selector: 'node[?hasNext]',
                style: { 'border-width': 3, 'border-color': '#60a5fa' },
            },
            {
                selector: 'node[?hasDrift]',
                style: { 'border-width': 3, 'border-color': '#f59e0b' },
            },
            {
                selector: 'node:selected',
                // Slightly thicker so selection is distinct from drift/next border.
                style: { 'border-width': 4, 'border-color': '#ffffff' },
            },
            {
                selector: 'node:active',
                style: { 'overlay-opacity': 0.1 },
            },
            {
                selector: 'node.dimmed',
                style: { 'opacity': 0.2 },
            },
            {
                selector: 'node.highlighted',
                style: { 'border-width': 2, 'border-color': '#ffffff' },
            },
            {
                selector: 'edge',
                style: {
                    'width': 1.5,
                    'line-color': '#6b7280',
                    'target-arrow-color': '#6b7280',
                    'target-arrow-shape': 'triangle',
                    'curve-style': 'bezier',
                    'arrow-scale': 0.8,
                },
            },
            {
                selector: 'edge.dimmed',
                style: { 'opacity': 0.1 },
            },
            {
                selector: 'edge:selected',
                style: {
                    'line-color': '#ffffff',
                    'target-arrow-color': '#ffffff',
                },
            },
        ] as cytoscape.StylesheetStyle[],
        layout: LAYOUT_OPTIONS as Parameters<cytoscape.Core['layout']>[0],
        minZoom: 0.2,
        maxZoom: 3,
    });

    wireInteractions();
}

// ── Hover tooltip ────────────────────────────────────────────────────────────

const tooltip = document.getElementById('tooltip')!;

function showTooltip(clientX: number, clientY: number, node: GraphNode): void {
    const { id, status, specs, sampled, audited, next, drift } = node;

    let html = `<div class="tt-title">${esc(id)}&nbsp;<span style="opacity:.6">[${esc(status)}]</span></div>`;

    if (specs) {
        const { implemented, open, deferred } = specs;
        const total = implemented + open + deferred;
        if (total > 0) {
            html += `<div class="tt-row">Specs:&nbsp;<strong>${implemented}</strong>✓&nbsp;${open}○&nbsp;${deferred}⊘</div>`;
        }
    }

    if (sampled || audited) {
        html += `<div class="tt-row">Sampled:&nbsp;${esc(fmtDate(sampled))}&nbsp;&nbsp;Audited:&nbsp;${esc(fmtDate(audited))}</div>`;
    }

    if (next || drift) {
        html += `<hr class="tt-sep">`;
        if (next)  html += `<div class="tt-row"><span style="opacity:.55">Next:&nbsp;</span>${esc(trunc(next, 120))}</div>`;
        if (drift) html += `<div class="tt-row"><span style="opacity:.55">Drift:&nbsp;</span>${esc(trunc(drift, 120))}</div>`;
    }

    tooltip.innerHTML = html;
    tooltip.style.display = 'block';
    positionTooltip(clientX, clientY);
}

function positionTooltip(clientX: number, clientY: number): void {
    const vw = document.documentElement.clientWidth;
    const vh = document.documentElement.clientHeight;
    tooltip.style.left = `${Math.min(clientX + 14, vw - tooltip.offsetWidth - 8)}px`;
    tooltip.style.top  = `${Math.min(clientY + 14, vh - tooltip.offsetHeight - 8)}px`;
}

function hideTooltip(): void {
    tooltip.style.display = 'none';
}

// ── Sidebar panel ────────────────────────────────────────────────────────────

const sidePanel   = document.getElementById('panel')!;
const panelTitle  = document.getElementById('panel-title')!;
const panelBody   = document.getElementById('panel-body')!;
const panelClose  = document.getElementById('panel-close')!;
const cyContainer = document.getElementById('cy')!;

panelClose.addEventListener('click', closePanel);

function openPanel(node: GraphNode): void {
    const { id, status, specs, specFile, lldFile, sampled, audited, next, drift } = node;

    panelTitle.textContent = id;

    const color = colorForStatus(status);
    let html = `<span class="badge" style="background:${color}">${esc(status)}</span>`;

    if (specs) {
        const { implemented, open, deferred } = specs;
        const total = implemented + open + deferred;
        if (total > 0) {
            const pct = Math.round((implemented / total) * 100);
            const barColor = open > 0 ? '#f59e0b' : '#22c55e';
            html += `<div class="sec">
              <div class="sec-title">Specs</div>
              <div class="prog-wrap">
                <div class="prog-fill" style="width:${pct}%;background:${barColor}"></div>
              </div>
              <div class="spec-row">
                <span>✓&nbsp;${implemented}</span>
                <span>○&nbsp;${open}</span>
                <span>⊘&nbsp;${deferred}</span>
                <span style="margin-left:auto">${implemented}/${total}</span>
              </div>
            </div>`;
        }
    }

    if (sampled || audited) {
        html += `<div class="sec"><div class="meta">
          <span class="meta-k">Sampled</span><span>${esc(fmtDate(sampled))}</span>
          <span class="meta-k">Audited</span><span>${esc(fmtDate(audited))}</span>
        </div></div>`;
    }

    html += `<div class="sec">
      <button class="btn-open" data-action="open-arrow">Open Arrow Doc</button>`;
    if (specFile) {
        html += `<button class="btn-open" data-action="open-spec">Open Spec File</button>`;
    }
    if (lldFile) {
        html += `<button class="btn-open" data-action="open-lld">Open LLD</button>`;
    }
    html += `</div>`;

    if (next) {
        html += `<div class="sec"><div class="sec-title">Next</div><div class="prose">${esc(next)}</div></div>`;
    }
    if (drift) {
        html += `<div class="sec"><div class="sec-title">Drift</div><div class="prose">${esc(drift)}</div></div>`;
    }

    panelBody.innerHTML = html;

    panelBody.querySelectorAll<HTMLButtonElement>('.btn-open').forEach((btn) => {
        btn.addEventListener('click', () => {
            const action = btn.dataset['action'];
            if (action === 'open-arrow') {
                vscode.postMessage({ type: 'open', segmentId: id });
            } else if (action === 'open-spec' && specFile) {
                vscode.postMessage({ type: 'openFile', path: specFile });
            } else if (action === 'open-lld' && lldFile) {
                vscode.postMessage({ type: 'openFile', path: lldFile });
            }
        });
    });

    sidePanel.classList.add('open');
    cy?.resize();
}

function closePanel(): void {
    sidePanel.classList.remove('open');
    cy?.resize();
}

// ── Context menu ─────────────────────────────────────────────────────────────

const ctxMenu  = document.getElementById('ctxmenu')!;
const ctxArrow = document.getElementById('ctx-arrow')!;
const ctxSpec  = document.getElementById('ctx-spec')!;
const ctxLld   = document.getElementById('ctx-lld')!;
const ctxCopy  = document.getElementById('ctx-copy')!;

let ctxNode: GraphNode | null = null;

function showCtxMenu(clientX: number, clientY: number, node: GraphNode): void {
    ctxNode = node;
    ctxSpec.classList.toggle('hidden', !node.specFile);
    ctxLld.classList.toggle('hidden', !node.lldFile);

    ctxMenu.style.display = 'block';
    const vw = document.documentElement.clientWidth;
    const vh = document.documentElement.clientHeight;
    ctxMenu.style.left = `${Math.min(clientX, vw - ctxMenu.offsetWidth - 4)}px`;
    ctxMenu.style.top  = `${Math.min(clientY, vh - ctxMenu.offsetHeight - 4)}px`;
}

function hideCtxMenu(): void {
    ctxMenu.style.display = 'none';
    ctxNode = null;
}

ctxArrow.addEventListener('click', () => {
    if (ctxNode) vscode.postMessage({ type: 'open', segmentId: ctxNode.id });
    hideCtxMenu();
});
ctxSpec.addEventListener('click', () => {
    if (ctxNode?.specFile) vscode.postMessage({ type: 'openFile', path: ctxNode.specFile });
    hideCtxMenu();
});
ctxLld.addEventListener('click', () => {
    if (ctxNode?.lldFile) vscode.postMessage({ type: 'openFile', path: ctxNode.lldFile });
    hideCtxMenu();
});
ctxCopy.addEventListener('click', () => {
    if (ctxNode) void navigator.clipboard.writeText(ctxNode.id);
    hideCtxMenu();
});

document.addEventListener('click', () => hideCtxMenu());
document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') { hideCtxMenu(); closePanel(); }
});

// ── Toolbar: filter + search ─────────────────────────────────────────────────

const filterStatus = document.getElementById('filter-status') as HTMLSelectElement;
const searchInput  = document.getElementById('search') as HTMLInputElement;

function applyFilter(): void {
    if (!cy) return;
    const status = filterStatus.value;
    const q = searchInput.value.trim().toLowerCase();

    cy.batch(() => {
        cy!.nodes().forEach((n) => {
            const ns = (n.data('status') as string) ?? '';
            const id = n.id().toLowerCase();
            const statusOk = !status || ns === status;
            const searchOk = !q || id.includes(q);

            n.removeClass('dimmed highlighted');
            if (!statusOk) {
                n.addClass('dimmed');
            } else if (q && !searchOk) {
                n.addClass('dimmed');
            } else if (q && searchOk) {
                n.addClass('highlighted');
            }
        });

        cy!.edges().forEach((e) => {
            const srcDimmed = e.source().hasClass('dimmed');
            const tgtDimmed = e.target().hasClass('dimmed');
            e.toggleClass('dimmed', srcDimmed || tgtDimmed);
        });
    });
}

filterStatus.addEventListener('change', applyFilter);
searchInput.addEventListener('input', applyFilter);

document.getElementById('btn-clear')!.addEventListener('click', () => {
    searchInput.value = '';
    applyFilter();
});

document.getElementById('btn-fit')!.addEventListener('click', () => cy?.fit());

// ── Cytoscape event wiring ────────────────────────────────────────────────────

function wireInteractions(): void {
    if (!cy) return;

    // Hover tooltip
    cy.on('mouseover', 'node', (evt) => {
        const node = evt.target.data('nodeData') as GraphNode;
        const me = evt.originalEvent as MouseEvent;
        showTooltip(me.clientX, me.clientY, node);
    });
    cy.on('mousemove', 'node', (evt) => {
        const me = evt.originalEvent as MouseEvent;
        positionTooltip(me.clientX, me.clientY);
    });
    cy.on('mouseout', 'node', hideTooltip);

    // Click → sidebar. Dual listener (tap + click) for macOS trackpad inside WebView.
    // Debounced so both events from the same gesture don't fire twice.
    let lastOpenTs = 0;
    const openSidebar = (evt: cytoscape.EventObject) => {
        const now = Date.now();
        if (now - lastOpenTs < 100) return;
        lastOpenTs = now;
        const node = evt.target.data('nodeData') as GraphNode;
        hideCtxMenu();
        hideTooltip();
        openPanel(node);
    };
    cy.on('tap',   'node', openSidebar);
    cy.on('click', 'node', openSidebar);

    // Right-click → context menu
    cy.on('cxttap', 'node', (evt) => {
        const node = evt.target.data('nodeData') as GraphNode;
        const me = evt.originalEvent as MouseEvent;
        hideTooltip();
        showCtxMenu(me.clientX, me.clientY, node);
    });
}

// ── Message bridge ────────────────────────────────────────────────────────────

window.addEventListener(
    'message',
    (event: MessageEvent<{ type: string; payload: GraphPayload }>) => {
        if (event.data.type === 'graph') {
            render(event.data.payload);
        }
    },
);
