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
    specPrefix?: string;
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
    UNMAPPED: '#6B7280',
    MAPPED:   '#6366F1',
    AUDITED:  '#F59E0B',
    OK:       '#81B29A',
    MERGED:   '#14B8A6',
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
let _allSegmentIds: string[] = [];
let _addSegBlocks: string[] = [];
let _addSegChildren: string[] = [];

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
    _allSegmentIds = payload.nodes.map(n => n.id).sort();
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
        // Refresh sidebar with updated node data if it's open
        if (currentPanelNode && sidePanel.classList.contains('open')) {
            const updated = payload.nodes.find(n => n.id === currentPanelNode!.id);
            if (updated) openPanel(updated, editMode);
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
                    'font-family': '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
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
                style: { 'border-width': 3, 'border-color': '#E07A5F' },
            },
            {
                selector: 'node[?hasDrift]',
                style: { 'border-width': 3, 'border-color': '#F59E0B' },
            },
            {
                selector: 'node:selected',
                style: { 'border-width': 4, 'border-color': '#E07A5F' },
            },
            {
                selector: 'node:active',
                style: { 'overlay-opacity': 0.08 },
            },
            {
                selector: 'node.dimmed',
                style: { 'opacity': 0.15 },
            },
            {
                selector: 'node.highlighted',
                style: { 'border-width': 2, 'border-color': '#E07A5F' },
            },
            {
                selector: 'edge',
                style: {
                    'width': 1.5,
                    'line-color': '#3D4663',
                    'target-arrow-color': '#3D4663',
                    'target-arrow-shape': 'triangle',
                    'curve-style': 'bezier',
                    'arrow-scale': 0.8,
                },
            },
            {
                selector: 'edge[kind = "child"]',
                style: {
                    'width': 1,
                    'line-color': '#252A40',
                    'line-style': 'dashed',
                    'target-arrow-shape': 'none',
                    'curve-style': 'bezier',
                },
            },
            {
                selector: 'edge.dimmed',
                style: { 'opacity': 0.08 },
            },
            {
                selector: 'edge:selected',
                style: {
                    'line-color': '#E07A5F',
                    'target-arrow-color': '#E07A5F',
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

// ── Edit mode state ──────────────────────────────────────────────────────────

let editMode = false;
const btnEdit = document.getElementById('btn-edit') as HTMLButtonElement;

btnEdit.addEventListener('click', () => {
    if (!currentPanelNode) return;
    editMode = !editMode;
    btnEdit.textContent = editMode ? '● Done' : '✎ Edit';
    btnEdit.classList.toggle('active', editMode);
    openPanel(currentPanelNode, editMode);
});

// ── Mutation feedback banner ──────────────────────────────────────────────────

const mutBanner = document.getElementById('mut-banner')!;
let mutBannerTimer: ReturnType<typeof setTimeout> | undefined;

function showMutBanner(ok: boolean, message: string): void {
    clearTimeout(mutBannerTimer);
    mutBanner.textContent = message;
    mutBanner.className = ok ? 'ok' : 'err';
    mutBanner.style.display = 'block';
    mutBannerTimer = setTimeout(() => { mutBanner.style.display = 'none'; }, 3000);
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
    openPanel(node.data('nodeData') as GraphNode, editMode);
}

function openPanel(node: GraphNode, edit = false): void {
    currentPanelNode = node;
    editMode = edit;
    btnEdit.textContent = editMode ? '● Done' : '✎ Edit';
    btnEdit.classList.toggle('active', editMode);
    const { id, status, specs, specItems, specFile, lldFile, specPrefix, sampled, audited, next, drift } = node;

    panelTitle.textContent = id;

    let html = editMode
        ? `<select class="edit-select" id="edit-status" data-seg-id="${esc(id)}">
            ${['UNMAPPED','MAPPED','AUDITED','OK','MERGED'].map(s =>
              `<option value="${s}"${s === status ? ' selected' : ''}>${s}</option>`
            ).join('')}
           </select>`
        : `<span class="badge badge-${esc(status)}">${esc(status)}</span>`;

    // ── Hierarchy chips (parent / children) ─────────────────────────────────
    if (node.parent) {
        html += `<div class="sec"><div class="sec-title">Parent</div><div class="chips">`;
        if (editMode) {
            html += `<div class="chip chip-rm" data-nav="${esc(node.parent)}">${esc(node.parent)}<button class="chip-x" data-rm-kind="parent" data-rm-seg="${esc(id)}" data-rm-target="${esc(node.parent)}">×</button></div>`;
        } else {
            html += `<button class="chip" data-nav="${esc(node.parent)}">${esc(node.parent)}</button>`;
        }
        html += `</div></div>`;
    }
    if (node.children && node.children.length > 0) {
        html += `<div class="sec"><div class="sec-title">Children</div><div class="chips">`;
        for (const c of node.children) {
            if (editMode) {
                html += `<div class="chip chip-rm" data-nav="${esc(c)}">${esc(c)}<button class="chip-x" data-rm-kind="children" data-rm-seg="${esc(id)}" data-rm-target="${esc(c)}">×</button></div>`;
            } else {
                html += `<button class="chip" data-nav="${esc(c)}">${esc(c)}</button>`;
            }
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
                if (editMode) {
                    html += `<div class="chip chip-rm" data-nav="${esc(n.id())}">${esc(n.id())}<button class="chip-x" data-rm-kind="blockedBy" data-rm-seg="${esc(id)}" data-rm-target="${esc(n.id())}">×</button></div>`;
                } else {
                    html += `<button class="chip" data-nav="${esc(n.id())}">${esc(n.id())}</button>`;
                }
            });
            html += `</div>`;
        }
        if ((successors?.length ?? 0) > 0) {
            html += `<div class="sec-title"${predecessors?.length ? ' style="margin-top:8px"' : ''}>Blocks</div><div class="chips">`;
            successors!.forEach((n) => {
                if (editMode) {
                    html += `<div class="chip chip-rm" data-nav="${esc(n.id())}">${esc(n.id())}<button class="chip-x" data-rm-kind="blocks" data-rm-seg="${esc(id)}" data-rm-target="${esc(n.id())}">×</button></div>`;
                } else {
                    html += `<button class="chip" data-nav="${esc(n.id())}">${esc(n.id())}</button>`;
                }
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
            const barColor = open > 0 ? '#F59E0B' : '#81B29A';
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
                    const statusVal = item.marker === 'done' ? 'implemented' : item.marker === 'open' ? 'open' : 'deferred';
                    const idBtn = item.id && item.line !== undefined && node.specFile
                        ? `<button class="si-id" data-path="${esc(node.specFile)}" data-line="${item.line}">${esc(item.id)}</button>`
                        : '';
                    const marker = editMode && item.id && item.line !== undefined && node.specFile
                        ? `<button class="si-toggle" data-id="${esc(item.id)}" data-file="${esc(node.specFile)}" data-line="${item.line}" data-status="${statusVal}" title="Cycle status"><span class="si-m ${cls}">${sym}</span></button>`
                        : `<span class="si-m ${cls}">${sym}</span>`;
                    html += `<div class="si">${marker}${idBtn}<span class="si-t">${esc(trunc(item.text, 100))}</span></div>`;
                }
                html += `</div>`;
            }
            // Add-spec form (edit mode only)
            if (editMode) {
                html += `<button class="add-spec-btn" id="btn-add-spec">+ Add spec</button>
                  <div class="add-spec-form" id="add-spec-form">
                    <div class="add-spec-row">
                      <input class="edit-input" id="new-spec-id" placeholder="${esc(specPrefix ?? id.toUpperCase())}-001" style="flex:0 0 110px">
                      <input class="edit-input" id="new-spec-text" placeholder="spec text…">
                    </div>
                    <div class="add-spec-row">
                      <button class="btn-save" id="btn-submit-spec" style="margin-top:0">Add</button>
                      <span class="add-spec-err" id="spec-id-err"></span>
                    </div>
                  </div>`;
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
    if (editMode) {
        html += `<div class="sec"><div class="sec-title">Next</div>
          <textarea class="edit-textarea" id="edit-next" data-seg-id="${esc(id)}">${esc(next ?? '')}</textarea></div>
          <div class="sec"><div class="sec-title">Drift</div>
          <textarea class="edit-textarea" id="edit-drift" data-seg-id="${esc(id)}">${esc(drift ?? '')}</textarea></div>
          <div class="sec"><button class="btn-save" id="btn-save-meta">Save</button></div>`;
    } else {
        if (next)  html += `<div class="sec"><div class="sec-title">Next</div><div class="prose">${esc(next)}</div></div>`;
        if (drift) html += `<div class="sec"><div class="sec-title">Drift</div><div class="prose">${esc(drift)}</div></div>`;
    }

    panelBody.innerHTML = html;
    sidePanel.classList.add('open');
    cy?.resize();
}

// Single delegated click handler for the whole panel body.
panelBody.addEventListener('click', (e) => {
    const t = e.target as HTMLElement;

    // Edit-mode: remove connection chip
    const chipX = t.closest<HTMLButtonElement>('.chip-x');
    if (chipX && editMode) {
        vscode.postMessage({
            type: 'removeConnection',
            kind: chipX.dataset['rmKind'] as 'blocks' | 'blockedBy' | 'children' | 'parent',
            segmentId: chipX.dataset['rmSeg']!,
            target: chipX.dataset['rmTarget']!,
        });
        return;
    }

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
        return;
    }

    // Edit-mode: spec status cycle
    const siToggle = t.closest<HTMLButtonElement>('.si-toggle');
    if (siToggle && editMode) {
        const cur = siToggle.dataset['status'] as 'open' | 'implemented' | 'deferred';
        const next: 'open' | 'implemented' | 'deferred' =
            cur === 'open' ? 'implemented' : cur === 'implemented' ? 'deferred' : 'open';
        vscode.postMessage({
            type: 'updateSpecStatus',
            specFile: siToggle.dataset['file']!,
            specId: siToggle.dataset['id']!,
            line: parseInt(siToggle.dataset['line'] ?? '0', 10),
            newStatus: next,
        });
        // Optimistic UI update
        const newCls = next === 'implemented' ? 'si-done' : next === 'open' ? 'si-open' : 'si-def';
        const newSym = next === 'implemented' ? '✓' : next === 'open' ? '○' : '⊘';
        const span = siToggle.querySelector<HTMLSpanElement>('.si-m');
        if (span) { span.className = `si-m ${newCls}`; span.textContent = newSym; }
        siToggle.dataset['status'] = next;
        return;
    }

    // Edit-mode: toggle add-spec form
    if ((t as HTMLElement).id === 'btn-add-spec') {
        panelBody.querySelector<HTMLElement>('#add-spec-form')?.classList.toggle('open');
        return;
    }

    // Edit-mode: submit new spec
    if ((t as HTMLElement).id === 'btn-submit-spec' && editMode && currentPanelNode) {
        const specIdInput = panelBody.querySelector<HTMLInputElement>('#new-spec-id');
        const specTextInput = panelBody.querySelector<HTMLInputElement>('#new-spec-text');
        const errSpan = panelBody.querySelector<HTMLElement>('#spec-id-err');
        const specId = specIdInput?.value.trim() ?? '';
        const text = specTextInput?.value.trim() ?? '';
        const prefix = currentPanelNode.specPrefix ?? currentPanelNode.id.toUpperCase();
        const valid = /^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$/.test(specId)
            && specId.startsWith(prefix + '-')
            && /\d/.test(specId);
        if (!valid) { if (errSpan) errSpan.textContent = `Must start with ${prefix}- and include a digit`; return; }
        if (!text)  { if (errSpan) errSpan.textContent = 'Text is required'; return; }
        if (errSpan) errSpan.textContent = '';
        vscode.postMessage({ type: 'addSpec', specFile: currentPanelNode.specFile, segmentId: currentPanelNode.id, specId, text, specPrefix: currentPanelNode.specPrefix });
        if (specIdInput) specIdInput.value = '';
        if (specTextInput) specTextInput.value = '';
        panelBody.querySelector<HTMLElement>('#add-spec-form')?.classList.remove('open');
        return;
    }

    // Edit-mode: save next/drift
    if ((t as HTMLElement).id === 'btn-save-meta' && editMode && currentPanelNode) {
        const nextTA  = panelBody.querySelector<HTMLTextAreaElement>('#edit-next');
        const driftTA = panelBody.querySelector<HTMLTextAreaElement>('#edit-drift');
        vscode.postMessage({
            type: 'updateSegmentMeta',
            segmentId: currentPanelNode.id,
            next:  nextTA?.value  ?? '',
            drift: driftTA?.value ?? '',
        });
        return;
    }
});

// Edit-mode: status dropdown change
panelBody.addEventListener('change', (e) => {
    const t = e.target as HTMLElement;
    if (t.id === 'edit-status' && editMode && currentPanelNode) {
        vscode.postMessage({
            type: 'updateSegmentStatus',
            segmentId: currentPanelNode.id,
            newStatus: (t as HTMLSelectElement).value,
        });
    }
});

function closePanel(): void {
    currentPanelNode = null;
    editMode = false;
    btnEdit.textContent = '✎ Edit';
    btnEdit.classList.remove('active');
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

    // Tap+click: debounce co-fire from macOS trackpad.
    let lastOpenTs = 0;
    const openSidebar = (evt: cytoscape.EventObject) => {
        const now = Date.now();
        if (now - lastOpenTs < 100) return;
        lastOpenTs = now;
        hideCtxMenu(); hideTooltip();
        openPanel(evt.target.data('nodeData') as GraphNode);
    };
    cy.on('tap',   'node', openSidebar);
    cy.on('click', 'node', openSidebar);

    // Double-click opens directly in edit mode (re-renders the already-open panel).
    cy.on('dbltap', 'node', (evt) => {
        hideCtxMenu(); hideTooltip();
        openPanel(evt.target.data('nodeData') as GraphNode, true);
    });

    cy.on('cxttap', 'node', (evt) => {
        const me = evt.originalEvent as MouseEvent;
        hideTooltip();
        showCtxMenu(me.clientX, me.clientY, evt.target.data('nodeData') as GraphNode);
    });
}

// ── Add-segment overlay ───────────────────────────────────────────────────────

const addSegOverlay = document.getElementById('add-seg-overlay')!;

function refreshOverlayChips(): void {
    const blocksChips   = document.getElementById('seg-blocks-chips')!;
    const childrenChips = document.getElementById('seg-children-chips')!;

    blocksChips.innerHTML = _addSegBlocks.map(id =>
        `<div class="chip chip-rm">${esc(id)}<button class="chip-x overlay-chip-x" data-rm-from="blocks" data-rm-id="${esc(id)}">×</button></div>`
    ).join('');
    childrenChips.innerHTML = _addSegChildren.map(id =>
        `<div class="chip chip-rm">${esc(id)}<button class="chip-x overlay-chip-x" data-rm-from="children" data-rm-id="${esc(id)}">×</button></div>`
    ).join('');

    // Remove already-selected IDs from each select's options
    const blocksSelect   = document.getElementById('seg-blocks-select')   as HTMLSelectElement;
    const childrenSelect = document.getElementById('seg-children-select') as HTMLSelectElement;
    if (!blocksSelect || !childrenSelect) return;

    for (const opt of Array.from(blocksSelect.options)) {
        if (!opt.value) continue;
        opt.hidden = _addSegBlocks.includes(opt.value) || _addSegChildren.includes(opt.value);
    }
    for (const opt of Array.from(childrenSelect.options)) {
        if (!opt.value) continue;
        opt.hidden = _addSegChildren.includes(opt.value) || _addSegBlocks.includes(opt.value);
    }
}

function populateOverlaySelects(): void {
    const blocksSelect   = document.getElementById('seg-blocks-select')   as HTMLSelectElement;
    const childrenSelect = document.getElementById('seg-children-select') as HTMLSelectElement;
    if (!blocksSelect || !childrenSelect) return;

    for (const sel of [blocksSelect, childrenSelect]) {
        while (sel.options.length > 1) sel.remove(1);
        for (const id of _allSegmentIds) {
            const opt = document.createElement('option');
            opt.value = id;
            opt.textContent = id;
            sel.appendChild(opt);
        }
        sel.value = '';
    }
}

document.getElementById('btn-add-seg')!.addEventListener('click', () => {
    (document.getElementById('seg-id-input') as HTMLInputElement).value = '';
    (document.getElementById('seg-detail-input') as HTMLInputElement).value = '';
    (document.getElementById('seg-add-err') as HTMLElement).textContent = '';
    _addSegBlocks   = [];
    _addSegChildren = [];
    populateOverlaySelects();
    refreshOverlayChips();
    addSegOverlay.classList.add('open');
});

document.getElementById('seg-blocks-select')!.addEventListener('change', (e) => {
    const sel = e.target as HTMLSelectElement;
    const id = sel.value;
    if (id && !_addSegBlocks.includes(id)) {
        _addSegBlocks.push(id);
        refreshOverlayChips();
    }
    sel.value = '';
});

document.getElementById('seg-children-select')!.addEventListener('change', (e) => {
    const sel = e.target as HTMLSelectElement;
    const id = sel.value;
    if (id && !_addSegChildren.includes(id)) {
        _addSegChildren.push(id);
        refreshOverlayChips();
    }
    sel.value = '';
});

// Delegated removal of chips inside the overlay
document.getElementById('add-seg-box')!.addEventListener('click', (e) => {
    const btn = (e.target as HTMLElement).closest<HTMLButtonElement>('.overlay-chip-x');
    if (!btn) return;
    const from = btn.dataset['rmFrom'];
    const id   = btn.dataset['rmId'];
    if (!id) return;
    if (from === 'blocks')   _addSegBlocks   = _addSegBlocks.filter(x => x !== id);
    if (from === 'children') _addSegChildren = _addSegChildren.filter(x => x !== id);
    refreshOverlayChips();
});

document.getElementById('btn-seg-cancel')!.addEventListener('click', () => {
    addSegOverlay.classList.remove('open');
});

document.getElementById('btn-seg-submit')!.addEventListener('click', () => {
    const segId   = (document.getElementById('seg-id-input')    as HTMLInputElement).value.trim();
    const status  = (document.getElementById('seg-status-select') as HTMLSelectElement).value;
    const detail  = (document.getElementById('seg-detail-input') as HTMLInputElement).value.trim();
    const errEl   = document.getElementById('seg-add-err')!;
    if (!segId || !/^[a-z][a-z0-9-]*$/.test(segId)) {
        errEl.textContent = 'ID: lowercase letters, digits, hyphens only'; return;
    }
    if (!detail) { errEl.textContent = 'Detail path is required'; return; }
    errEl.textContent = '';
    addSegOverlay.classList.remove('open');
    vscode.postMessage({
        type: 'addSegment',
        segmentId: segId,
        status,
        detail,
        blocks:   [..._addSegBlocks],
        children: [..._addSegChildren],
    });
});

addSegOverlay.addEventListener('click', (e) => {
    if (e.target === addSegOverlay) addSegOverlay.classList.remove('open');
});

// ── Message bridge ────────────────────────────────────────────────────────────

window.addEventListener(
    'message',
    (event: MessageEvent<{ type: string; payload: GraphPayload; message?: string }>) => {
        if (event.data.type === 'graph') {
            render(event.data.payload);
        } else if (event.data.type === 'mutationOk') {
            showMutBanner(true, event.data.message ?? 'Done');
        } else if (event.data.type === 'mutationError') {
            showMutBanner(false, event.data.message ?? 'Error');
        }
    },
);
