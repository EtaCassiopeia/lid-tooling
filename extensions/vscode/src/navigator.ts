// WebView panel that renders the arrow-of-intent DAG.
//
// The extension host reads docs/arrows/index.yaml and docs/intent/ directly
// (no LSP round-trip) and posts a serialised GraphPayload to the webview.
// The webview renders it with Cytoscape + dagre and posts node-interaction
// events back so the host can open the corresponding artifacts.

import * as crypto from 'crypto';
import * as fs from 'fs';
import * as path from 'path';

import * as vscode from 'vscode';
import * as yaml from 'js-yaml';

const VIEW_TYPE = 'lid.intentNavigator';

// ── Shared types (must stay in sync with src/webview/navigator.ts) ───────────

interface ArrowEntry {
    status?: string;
    detail?: string;
    blocks?: string[];
    next?: string;
    drift?: string;
    sampled?: string;
    audited?: string;
}

interface ArrowIndex {
    arrows?: Record<string, ArrowEntry>;
}

interface SpecCounts {
    implemented: number;
    open: number;
    deferred: number;
}

interface SpecInfo {
    counts: SpecCounts;
    specFile?: string;
    lldFile?: string;
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

type WebviewMessage =
    | { type: 'open'; segmentId: string }
    | { type: 'openFile'; path: string };

// ── NavigatorPanel ────────────────────────────────────────────────────────────

export class NavigatorPanel {
    public static currentPanel: NavigatorPanel | undefined;

    private readonly _panel: vscode.WebviewPanel;
    private readonly _workspaceRoot: string;
    private readonly _disposables: vscode.Disposable[] = [];

    // ── Public factory methods ──────────────────────────────────────────────

    public static createOrShow(extensionUri: vscode.Uri, workspaceRoot: string): void {
        const column = vscode.window.activeTextEditor
            ? vscode.ViewColumn.Beside
            : vscode.ViewColumn.One;

        if (NavigatorPanel.currentPanel) {
            NavigatorPanel.currentPanel._panel.reveal(column);
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            VIEW_TYPE,
            'LID Intent Navigator',
            column,
            webviewOptions(extensionUri),
        );
        NavigatorPanel.currentPanel = new NavigatorPanel(panel, extensionUri, workspaceRoot);
    }

    public static revive(
        panel: vscode.WebviewPanel,
        extensionUri: vscode.Uri,
        workspaceRoot: string,
    ): void {
        NavigatorPanel.currentPanel = new NavigatorPanel(panel, extensionUri, workspaceRoot);
    }

    // ── Constructor ─────────────────────────────────────────────────────────

    private constructor(
        panel: vscode.WebviewPanel,
        extensionUri: vscode.Uri,
        workspaceRoot: string,
    ) {
        this._panel = panel;
        this._workspaceRoot = workspaceRoot;

        this._panel.webview.html = buildHtml(extensionUri, this._panel.webview);
        this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

        this._panel.webview.onDidReceiveMessage(
            (msg: WebviewMessage) => {
                if (msg.type === 'openFile') {
                    this._openDocAtPath(msg.path);
                } else {
                    const entry = this._indexEntry(msg.segmentId);
                    const detailFile = entry?.detail ?? `${msg.segmentId}.md`;
                    this._openDocAtPath(
                        path.join(this._workspaceRoot, 'docs', 'arrows', detailFile),
                    );
                }
            },
            null,
            this._disposables,
        );

        // Re-post graph whenever index.yaml changes on disk.
        const watcher = vscode.workspace.createFileSystemWatcher(
            new vscode.RelativePattern(workspaceRoot, 'docs/arrows/index.yaml'),
        );
        const repost = () => this._postGraph();
        watcher.onDidChange(repost, null, this._disposables);
        watcher.onDidCreate(repost, null, this._disposables);
        this._disposables.push(watcher);

        setTimeout(() => this._postGraph(), 150);
    }

    // ── Private helpers ─────────────────────────────────────────────────────

    private _openDocAtPath(docPath: string): void {
        void Promise.resolve(vscode.workspace.openTextDocument(docPath))
            .then((doc) => vscode.window.showTextDocument(doc, vscode.ViewColumn.One))
            .catch((err: unknown) => {
                void vscode.window.showErrorMessage(
                    `LID Navigator: could not open ${docPath} — ${String(err)}`,
                );
            });
    }

    private _loadIndex(): ArrowIndex {
        const indexPath = path.join(this._workspaceRoot, 'docs', 'arrows', 'index.yaml');
        try {
            return yaml.load(fs.readFileSync(indexPath, 'utf8')) as ArrowIndex;
        } catch {
            return {};
        }
    }

    private _indexEntry(segmentId: string): ArrowEntry | undefined {
        return this._loadIndex().arrows?.[segmentId];
    }

    private _buildSpecInfo(): Map<string, SpecInfo> {
        const intentDir = path.join(this._workspaceRoot, 'docs', 'intent');
        const result = new Map<string, SpecInfo>();

        let dirs: fs.Dirent[];
        try {
            dirs = fs.readdirSync(intentDir, { withFileTypes: true });
        } catch {
            return result;
        }

        for (const dir of dirs) {
            if (!dir.isDirectory()) continue;
            const segmentId = dir.name;
            const segmentDir = path.join(intentDir, segmentId);

            let files: fs.Dirent[];
            try {
                files = fs.readdirSync(segmentDir, { withFileTypes: true });
            } catch {
                continue;
            }

            let implemented = 0, open = 0, deferred = 0;
            let specFile: string | undefined;
            let lldFile: string | undefined;

            for (const f of files) {
                if (!f.isFile()) continue;
                const fullPath = path.join(segmentDir, f.name);
                if (f.name.endsWith('-specs.md')) {
                    if (!specFile) specFile = fullPath;
                    try {
                        for (const line of fs.readFileSync(fullPath, 'utf8').split('\n')) {
                            if (/^\s*-\s+\[x\]/i.test(line)) implemented++;
                            else if (/^\s*-\s+\[ \]/.test(line)) open++;
                            else if (/^\s*-\s+\[D\]/i.test(line)) deferred++;
                        }
                    } catch {
                        // skip unreadable file
                    }
                } else if (f.name.endsWith('-design.md')) {
                    lldFile = fullPath;
                }
            }

            result.set(segmentId, {
                counts: { implemented, open, deferred },
                specFile,
                lldFile,
            });
        }

        return result;
    }

    private _buildPayload(): GraphPayload {
        const arrows = this._loadIndex().arrows ?? {};
        const specInfo = this._buildSpecInfo();

        const nodes: GraphNode[] = Object.entries(arrows).map(([id, entry]) => {
            const info = specInfo.get(id);
            return {
                id,
                label: id,
                status: entry.status ?? 'UNMAPPED',
                next: entry.next,
                drift: entry.drift,
                sampled: entry.sampled,
                audited: entry.audited,
                specs: info?.counts,
                specFile: info?.specFile,
                lldFile: info?.lldFile,
            };
        });

        const edges: GraphEdge[] = [];
        const seen = new Set<string>();
        for (const [id, entry] of Object.entries(arrows)) {
            for (const target of entry.blocks ?? []) {
                const key = `${id}\x00${target}`;
                if (!seen.has(key) && target in arrows) {
                    seen.add(key);
                    edges.push({ source: id, target });
                }
            }
        }

        return { nodes, edges };
    }

    private _postGraph(): void {
        const payload = this._buildPayload();
        void this._panel.webview.postMessage({ type: 'graph', payload });
    }

    // ── Lifecycle ───────────────────────────────────────────────────────────

    public dispose(): void {
        NavigatorPanel.currentPanel = undefined;
        this._panel.dispose();
        for (const d of this._disposables) {
            d.dispose();
        }
        this._disposables.length = 0;
    }
}

// ── Serializer ────────────────────────────────────────────────────────────────

export class NavigatorPanelSerializer implements vscode.WebviewPanelSerializer {
    constructor(private readonly _extensionUri: vscode.Uri) {}

    async deserializeWebviewPanel(panel: vscode.WebviewPanel, _state: unknown): Promise<void> {
        const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        if (!workspaceRoot) {
            panel.dispose();
            return;
        }
        NavigatorPanel.revive(panel, this._extensionUri, workspaceRoot);
    }
}

// ── HTML builder ──────────────────────────────────────────────────────────────

function webviewOptions(
    extensionUri: vscode.Uri,
): vscode.WebviewOptions & vscode.WebviewPanelOptions {
    return {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [vscode.Uri.joinPath(extensionUri, 'media')],
    };
}

function buildHtml(extensionUri: vscode.Uri, webview: vscode.Webview): string {
    const scriptUri = webview.asWebviewUri(
        vscode.Uri.joinPath(extensionUri, 'media', 'navigator.js'),
    );
    const nonce = crypto.randomBytes(16).toString('hex');

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy"
        content="default-src 'none'; script-src 'nonce-${nonce}'; style-src 'unsafe-inline';">
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    html, body {
      width: 100%; height: 100%; overflow: hidden;
      background: var(--vscode-editor-background, #1e1e1e);
      color: var(--vscode-editor-foreground, #d4d4d4);
      font-family: var(--vscode-font-family, system-ui, sans-serif);
      font-size: 12px;
      display: flex; flex-direction: column;
    }
    /* ── Toolbar ── */
    #toolbar {
      height: 32px; flex-shrink: 0;
      display: flex; align-items: center; gap: 8px; padding: 0 10px;
      background: var(--vscode-editorGroupHeader-tabsBackground, #252526);
      border-bottom: 1px solid var(--vscode-editorGroup-border, #3c3c3c);
    }
    #toolbar button {
      background: var(--vscode-button-background, #0e639c);
      color: var(--vscode-button-foreground, #ffffff);
      border: none; border-radius: 2px; padding: 2px 10px;
      cursor: pointer; font-size: 11px;
    }
    #toolbar button:hover { background: var(--vscode-button-hoverBackground, #1177bb); }
    #btn-clear {
      background: transparent;
      color: var(--vscode-descriptionForeground, #9d9d9d);
      padding: 0 4px; font-size: 14px;
    }
    #toolbar label {
      display: flex; align-items: center; gap: 4px; font-size: 11px;
      color: var(--vscode-descriptionForeground, #9d9d9d);
    }
    #toolbar select, #toolbar input[type=text] {
      background: var(--vscode-input-background, #3c3c3c);
      color: var(--vscode-input-foreground, #cccccc);
      border: 1px solid var(--vscode-input-border, #555);
      border-radius: 2px; padding: 1px 6px; font-size: 11px; outline: none;
    }
    #search { width: 140px; }
    .tb-sep { width: 1px; height: 16px; background: var(--vscode-editorGroup-border, #3c3c3c); }
    /* ── Main area ── */
    #main { flex: 1; display: flex; overflow: hidden; min-height: 0; }
    #cy { flex: 1; min-width: 0; }
    /* ── Sidebar ── */
    #panel {
      width: 260px; flex-shrink: 0;
      display: none; flex-direction: column;
      border-left: 1px solid var(--vscode-editorGroup-border, #3c3c3c);
      background: var(--vscode-sideBar-background, #252526);
    }
    #panel.open { display: flex; }
    .ph {
      display: flex; justify-content: space-between; align-items: center;
      padding: 8px 10px; flex-shrink: 0;
      border-bottom: 1px solid var(--vscode-editorGroup-border, #3c3c3c);
      font-weight: 600; font-size: 12px;
    }
    .ph-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .ph-close { cursor: pointer; opacity: 0.6; padding: 0 2px; font-size: 14px; }
    .ph-close:hover { opacity: 1; }
    .pb { overflow-y: auto; padding: 10px; flex: 1; }
    .badge {
      display: inline-block; padding: 1px 8px; border-radius: 10px;
      font-size: 10px; font-weight: 600; color: #fff; margin-bottom: 10px;
    }
    .sec { margin-top: 12px; }
    .sec-title {
      font-size: 10px; text-transform: uppercase;
      letter-spacing: 0.06em; opacity: 0.55; margin-bottom: 4px;
    }
    .prog-wrap { height: 6px; border-radius: 3px; background: var(--vscode-input-background, #3c3c3c); overflow: hidden; margin: 4px 0; }
    .prog-fill { height: 100%; border-radius: 3px; }
    .spec-row { display: flex; gap: 8px; font-size: 10px; opacity: 0.75; margin-top: 2px; }
    .meta { display: grid; grid-template-columns: auto 1fr; gap: 2px 8px; font-size: 10px; opacity: 0.75; }
    .meta-k { opacity: 0.6; }
    .btn-open {
      display: block; width: 100%; margin-top: 5px;
      padding: 5px 8px; text-align: left;
      background: var(--vscode-button-secondaryBackground, #3a3d41);
      color: var(--vscode-button-secondaryForeground, #cccccc);
      border: none; border-radius: 3px; cursor: pointer; font-size: 11px;
    }
    .btn-open:hover { background: var(--vscode-button-secondaryHoverBackground, #45494e); }
    .prose {
      font-size: 11px; line-height: 1.55; opacity: 0.88;
      white-space: pre-wrap; word-break: break-word;
    }
    /* ── Tooltip ── */
    #tooltip {
      position: fixed; pointer-events: none;
      max-width: 290px;
      background: var(--vscode-editorHoverWidget-background, #252526);
      border: 1px solid var(--vscode-editorHoverWidget-border, #454545);
      border-radius: 4px; padding: 8px 10px;
      font-size: 11px; z-index: 999; display: none;
      line-height: 1.5; box-shadow: 0 2px 8px rgba(0,0,0,.4);
    }
    .tt-title { font-weight: 600; margin-bottom: 3px; }
    .tt-row { opacity: 0.8; margin-top: 1px; }
    .tt-sep { border: none; border-top: 1px solid var(--vscode-editorHoverWidget-border, #454545); margin: 5px 0; }
    /* ── Context menu ── */
    #ctxmenu {
      position: fixed;
      background: var(--vscode-menu-background, #252526);
      border: 1px solid var(--vscode-menu-border, #454545);
      border-radius: 4px; padding: 4px 0;
      font-size: 12px; z-index: 1000; display: none;
      min-width: 170px; box-shadow: 0 4px 12px rgba(0,0,0,.5);
    }
    .ctx-item { padding: 5px 14px; cursor: pointer; }
    .ctx-item:hover {
      background: var(--vscode-menu-selectionBackground, #094771);
      color: var(--vscode-menu-selectionForeground, #fff);
    }
    .ctx-item.hidden { display: none; }
    .ctx-sep { border-top: 1px solid var(--vscode-menu-border, #454545); margin: 3px 0; }
    /* ── Legend ── */
    #legend {
      position: fixed; bottom: 10px; left: 10px;
      background: var(--vscode-editorWidget-background, #252526);
      border: 1px solid var(--vscode-editorWidget-border, #454545);
      border-radius: 4px; padding: 7px 10px;
      font-size: 10px; opacity: 0.85; pointer-events: none; line-height: 1.8;
    }
    .leg-row { display: flex; align-items: center; gap: 5px; flex-wrap: wrap; }
    .sw { display: inline-block; width: 9px; height: 9px; border-radius: 50%; vertical-align: middle; }
    /* ── Empty ── */
    #empty {
      display: none; position: fixed; top: 50%; left: 50%;
      transform: translate(-50%,-50%); opacity: 0.4; font-size: 13px;
    }
  </style>
</head>
<body>
  <div id="toolbar">
    <button id="btn-fit">Fit</button>
    <div class="tb-sep"></div>
    <label>Status
      <select id="filter-status">
        <option value="">All</option>
        <option value="UNMAPPED">UNMAPPED</option>
        <option value="MAPPED">MAPPED</option>
        <option value="AUDITED">AUDITED</option>
        <option value="OK">OK</option>
        <option value="MERGED">MERGED</option>
      </select>
    </label>
    <label>Search
      <input id="search" type="text" placeholder="segment name…">
    </label>
    <button id="btn-clear" title="Clear search">✕</button>
  </div>

  <div id="main">
    <div id="cy"></div>
    <div id="panel">
      <div class="ph">
        <span class="ph-title" id="panel-title">—</span>
        <span class="ph-close" id="panel-close">✕</span>
      </div>
      <div class="pb" id="panel-body"></div>
    </div>
  </div>

  <div id="tooltip"></div>

  <div id="ctxmenu">
    <div class="ctx-item" id="ctx-arrow">Open Arrow Doc</div>
    <div class="ctx-item" id="ctx-spec">Open Spec File</div>
    <div class="ctx-item" id="ctx-lld">Open LLD</div>
    <div class="ctx-sep"></div>
    <div class="ctx-item" id="ctx-copy">Copy Segment ID</div>
  </div>

  <div id="legend">
    <div class="leg-row">
      <span class="sw" style="background:#6b7280"></span>UNMAPPED&nbsp;
      <span class="sw" style="background:#3b82f6"></span>MAPPED&nbsp;
      <span class="sw" style="background:#f59e0b"></span>AUDITED&nbsp;
      <span class="sw" style="background:#22c55e"></span>OK&nbsp;
      <span class="sw" style="background:#9ca3af"></span>MERGED
    </div>
    <div class="leg-row" style="margin-top:3px;opacity:0.8">
      <span class="sw" style="background:#f59e0b"></span>has drift &nbsp;
      <span class="sw" style="background:#3b82f6"></span>has next
    </div>
  </div>

  <div id="empty">No segments found in docs/arrows/index.yaml</div>
  <script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
}
