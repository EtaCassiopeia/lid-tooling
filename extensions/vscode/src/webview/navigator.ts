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

interface SpecItem {
    marker: 'done' | 'open' | 'deferred';
    id?: string;
    text: string;
    line?: number;
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
    specItems?: SpecItem[];
    specFile?: string;
    lldFile?: string;
    children?: string[];
    parent?: string;
}

interface GraphEdge {
    source: string;
    target: string;
    kind?: 'blocks' | 'child';
}

interface GraphPayload {
    nodes: GraphNode[];
    edges: GraphEdge[];
    clusters: Record<string, string[]>;
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

function fmtDate(s: string | undefined): string {
    if (!s) return '—';
    return s.replace(/T.*$/, '');
}

// ── Cytoscape instance ───────────────────────────────────────────────────────

let cy: cytoscape.Core | undefined;

let _clusters: Record<string, string[]> = {};

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
    _clusters = payload.clusters ?? {};
    const clusterSel = document.getElementById('filter-cluster') as HTMLSelectElement;
    if (clusterSel) {
        const prev = clusterSel.value;
        while (clusterSel.options.length > 1) clusterSel.remove(1);
        for (const name of Object.keys(_clusters).sort()) {
            const opt = document.createElement('option');
            opt.value = name;
            opt.textContent = name;
            clusterSel.appendChild(opt);
        }
        clusterSel.value = prev in _clusters ? prev : '';
    }

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
            data: { id: `e${i}`, source: e.source, target: e.target, kind: e.kind ?? 'blocks' },
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
                selector: 'edge[kind = "child"]',
                style: {
                    'width': 1,
                    'line-color': '#374151',
                    'line-style': 'dashed',
                    'target-arrow-shape': 'none',
                    'curve-style': 'bezier',
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

// ── Neighbourhood focus ───────────────────────────────────────────────────────

let focusedId: string | null = null;
const btnFocus = document.getElementById('btn-focus') as HTMLButtonElement;

function applyFocus(id: string): void {
    if (!cy) return;
    focusedId = id;
    const node = cy.getElementById(id);
    // Full ancestry + full descendancy chain (all nodes that can reach this, and
    // all nodes reachable from this), plus connecting edges within the set.
    const ancestors   = node.predecessors('node') as cytoscape.NodeCollection;
    const descendants = node.successors('node')   as cytoscape.NodeCollection;
    const focusedNodes = ancestors.union(descendants).union(node);
    const focusedEdges = focusedNodes.edgesWith(focusedNodes);
    cy.batch(() => {
        cy!.elements().addClass('dimmed');
        focusedNodes.union(focusedEdges).removeClass('dimmed');
    });
    btnFocus.classList.add('active');
    btnFocus.textContent = 'Unfocus';
}

function clearFocus(): void {
    if (!focusedId) return;
    focusedId = null;
    cy?.elements().removeClass('dimmed');
    applyFilter();
    btnFocus.classList.remove('active');
    btnFocus.textContent = 'Focus';
}

btnFocus.addEventListener('click', () => {
    if (focusedId) {
        clearFocus();
    } else if (currentPanelNode) {
        applyFocus(currentPanelNode.id);
    }
});

// ── Sidebar panel ────────────────────────────────────────────────────────────

const sidePanel  = document.getElementById('panel')!;
const panelTitle = document.getElementById('panel-title')!;
const panelBody  = document.getElementById('panel-body')!;

document.getElementById('panel-close')!.addEventListener('click', closePanel);

let currentPanelNode: GraphNode | null = null;

// Navigate to a node by ID: animate viewport, then refresh sidebar.
function navigateToNode(id: string): void {
    if (!cy) return;
    const node = cy.getElementById(id);
    if (!node.length) return;
    cy.animate({ center: { eles: node }, zoom: cy.zoom() }, { duration: 250 });
    openPanel(node.data('nodeData') as GraphNode);
}

function openPanel(node: GraphNode): void {
    currentPanelNode = node;
    const { id, status, specs, specItems, specFile, lldFile, sampled, audited, next, drift } = node;

    panelTitle.textContent = id;

    const color = colorForStatus(status);
    let html = `<span class="badge" style="background:${color}">${esc(status)}</span>`;

    // ── Hierarchy chips (parent / children) ─────────────────────────────────
    if (node.parent) {
        html += `<div class="sec"><div class="sec-title">Parent</div><div class="chips">`;
        html += `<button class="chip" data-nav="${esc(node.parent)}">${esc(node.parent)}</button>`;
        html += `</div></div>`;
    }
    if (node.children && node.children.length > 0) {
        html += `<div class="sec"><div class="sec-title">Children</div><div class="chips">`;
        for (const c of node.children) {
            html += `<button class="chip" data-nav="${esc(c)}">${esc(c)}</button>`;
        }
        html += `</div></div>`;
    }

    // ── Dependency chips (blocks edges only) ─────────────────────────────────
    const predecessors = cy?.getElementById(id).incomers('edge')
        .filter(e => e.data('kind') === 'blocks')
        .sources();
    const successors = cy?.getElementById(id).outgoers('edge')
        .filter(e => e.data('kind') === 'blocks')
        .targets();

    if ((predecessors?.length ?? 0) > 0 || (successors?.length ?? 0) > 0) {
        html += `<div class="sec">`;
        if ((predecessors?.length ?? 0) > 0) {
            html += `<div class="sec-title">Blocked by</div><div class="chips">`;
            predecessors!.forEach((n) => {
                html += `<button class="chip" data-nav="${esc(n.id())}">${esc(n.id())}</button>`;
            });
            html += `</div>`;
        }
        if ((successors?.length ?? 0) > 0) {
            html += `<div class="sec-title"${predecessors?.length ? ' style="margin-top:8px"' : ''}>Blocks</div><div class="chips">`;
            successors!.forEach((n) => {
                html += `<button class="chip" data-nav="${esc(n.id())}">${esc(n.id())}</button>`;
            });
            html += `</div>`;
        }
        html += `</div>`;
    }

    // ── Spec progress + expandable list ──────────────────────────────────────
    if (specs) {
        const { implemented, open, deferred } = specs;
        const total = implemented + open + deferred;
        if (total > 0) {
            const pct = Math.round((implemented / total) * 100);
            const barColor = open > 0 ? '#f59e0b' : '#22c55e';
            const hasItems = specItems && specItems.length > 0;
            html += `<div class="sec">
              <div class="sec-title-row">
                <span class="sec-title">Specs</span>
                ${hasItems ? `<button class="spec-toggle" data-count="${specItems!.length}">▾ ${specItems!.length} items</button>` : ''}
              </div>
              <div class="prog-wrap">
                <div class="prog-fill" style="width:${pct}%;background:${barColor}"></div>
              </div>
              <div class="spec-row">
                <span>✓&nbsp;${implemented}</span>
                <span>○&nbsp;${open}</span>
                <span>⊘&nbsp;${deferred}</span>
                <span style="margin-left:auto">${implemented}/${total}</span>
              </div>`;
            if (hasItems) {
                html += `<div class="spec-list" style="display:none">`;
                for (const item of specItems!) {
                    const cls = item.marker === 'done' ? 'si-done' : item.marker === 'open' ? 'si-open' : 'si-def';
                    const sym = item.marker === 'done' ? '✓' : item.marker === 'open' ? '○' : '⊘';
                    const idBtn = item.id && item.line !== undefined && node.specFile
                        ? `<button class="si-id" data-path="${esc(node.specFile)}" data-line="${item.line}">${esc(item.id)}</button>`
                        : '';
                    html += `<div class="si"><span class="si-m ${cls}">${sym}</span>${idBtn}<span class="si-t">${esc(trunc(item.text, 100))}</span></div>`;
                }
                html += `</div>`;
            }
            html += `</div>`;
        }
    }

    // ── Metadata ─────────────────────────────────────────────────────────────
    if (sampled || audited) {
        html += `<div class="sec"><div class="meta">
          <span class="meta-k">Sampled</span><span>${esc(fmtDate(sampled))}</span>
          <span class="meta-k">Audited</span><span>${esc(fmtDate(audited))}</span>
        </div></div>`;
    }

    // ── Action buttons ────────────────────────────────────────────────────────
    html += `<div class="sec">
      <button class="btn-open" data-action="open-arrow">Open Arrow Doc</button>`;
    if (specFile) html += `<button class="btn-open" data-action="open-spec">Open Spec File</button>`;
    if (lldFile)  html += `<button class="btn-open" data-action="open-lld">Open LLD</button>`;
    html += `</div>`;

    // ── Next / Drift ──────────────────────────────────────────────────────────
    if (next)  html += `<div class="sec"><div class="sec-title">Next</div><div class="prose">${esc(next)}</div></div>`;
    if (drift) html += `<div class="sec"><div class="sec-title">Drift</div><div class="prose">${esc(drift)}</div></div>`;

    panelBody.innerHTML = html;
    sidePanel.classList.add('open');
    cy?.resize();
}

// Single delegated click handler for the whole panel body.
panelBody.addEventListener('click', (e) => {
    const t = e.target as HTMLElement;

    const chip = t.closest<HTMLButtonElement>('.chip[data-nav]');
    if (chip?.dataset['nav']) { navigateToNode(chip.dataset['nav']); return; }

    const specIdBtn = t.closest<HTMLButtonElement>('.si-id');
    if (specIdBtn?.dataset['path']) {
        vscode.postMessage({ type: 'openFile', path: specIdBtn.dataset['path'], line: parseInt(specIdBtn.dataset['line'] ?? '0', 10) });
        return;
    }

    const btn = t.closest<HTMLButtonElement>('.btn-open[data-action]');
    if (btn?.dataset['action']) {
        const node = currentPanelNode;
        if (!node) return;
        const action = btn.dataset['action'];
        if (action === 'open-arrow') vscode.postMessage({ type: 'open', segmentId: node.id });
        else if (action === 'open-spec' && node.specFile) vscode.postMessage({ type: 'openFile', path: node.specFile });
        else if (action === 'open-lld'  && node.lldFile)  vscode.postMessage({ type: 'openFile', path: node.lldFile });
        return;
    }

    const toggle = t.closest<HTMLButtonElement>('.spec-toggle');
    if (toggle) {
        const list = panelBody.querySelector<HTMLElement>('.spec-list');
        if (!list) return;
        const open = list.style.display === 'none';
        list.style.display = open ? 'block' : 'none';
        toggle.textContent = `${open ? '▴' : '▾'} ${toggle.dataset['count'] ?? ''} items`;
    }
});

function closePanel(): void {
    currentPanelNode = null;
    clearFocus();
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

const filterStatus  = document.getElementById('filter-status')  as HTMLSelectElement;
const filterCluster = document.getElementById('filter-cluster') as HTMLSelectElement;
const searchInput   = document.getElementById('search') as HTMLInputElement;

function applyFilter(): void {
    if (!cy) return;
    const status  = filterStatus.value;
    const cluster = filterCluster.value;
    const q = searchInput.value.trim().toLowerCase();
    const clusterIds = cluster ? new Set(_clusters[cluster] ?? []) : null;

    cy.batch(() => {
        cy!.nodes().forEach((n) => {
            const ns = (n.data('status') as string) ?? '';
            const id = n.id();
            const statusOk  = !status  || ns === status;
            const clusterOk = !cluster || clusterIds!.has(id);
            const searchOk  = !q || id.toLowerCase().includes(q);

            n.removeClass('dimmed highlighted');
            if (!statusOk || !clusterOk) {
                n.addClass('dimmed');
            } else if (q && !searchOk) {
                n.addClass('dimmed');
            } else if (q && searchOk) {
                n.addClass('highlighted');
            }
        });

        cy!.edges().forEach((e) => {
            e.toggleClass('dimmed', e.source().hasClass('dimmed') || e.target().hasClass('dimmed'));
        });
    });
}

filterStatus.addEventListener('change', applyFilter);
filterCluster.addEventListener('change', applyFilter);
searchInput.addEventListener('input', applyFilter);

document.getElementById('btn-clear')!.addEventListener('click', () => {
    searchInput.value = '';
    applyFilter();
});

document.getElementById('btn-fit')!.addEventListener('click', () => cy?.fit());

// ── Cytoscape event wiring ────────────────────────────────────────────────────

function wireInteractions(): void {
    if (!cy) return;

    cy.on('mouseover', 'node', (evt) => {
        const node = evt.target.data('nodeData') as GraphNode;
        const me = evt.originalEvent as MouseEvent;
        showTooltip(me.clientX, me.clientY, node);
    });
    cy.on('mousemove', 'node', (evt) => {
        positionTooltip((evt.originalEvent as MouseEvent).clientX, (evt.originalEvent as MouseEvent).clientY);
    });
    cy.on('mouseout', 'node', hideTooltip);

    // Dual tap + click for macOS trackpad inside WebView; debounced to avoid double-fire.
    let lastOpenTs = 0;
    const openSidebar = (evt: cytoscape.EventObject) => {
        const now = Date.now();
        if (now - lastOpenTs < 100) return;
        lastOpenTs = now;
        hideCtxMenu();
        hideTooltip();
        openPanel(evt.target.data('nodeData') as GraphNode);
    };
    cy.on('tap',   'node', openSidebar);
    cy.on('click', 'node', openSidebar);

    cy.on('cxttap', 'node', (evt) => {
        const me = evt.originalEvent as MouseEvent;
        hideTooltip();
        showCtxMenu(me.clientX, me.clientY, evt.target.data('nodeData') as GraphNode);
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
