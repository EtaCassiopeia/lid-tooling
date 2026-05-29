// WebView panel that renders the arrow-of-intent DAG.
//
// The extension host reads docs/arrows/index.yaml directly (no LSP round-trip
// needed) and posts a serialised GraphPayload to the webview. The webview
// renders it with Cytoscape + dagre and posts node-click events back so the
// host can open the corresponding arrow detail doc.

import * as crypto from 'crypto';
import * as fs from 'fs';
import * as path from 'path';

import * as vscode from 'vscode';
import * as yaml from 'js-yaml';

const VIEW_TYPE = 'lid.intentNavigator';

interface ArrowEntry {
    status?: string;
    detail?: string;
    blocks?: string[];
}

interface ArrowIndex {
    arrows?: Record<string, ArrowEntry>;
}

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

interface WebviewMessage {
    type: 'open';
    segmentId: string;
}

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

    // Called by the WebviewPanelSerializer to restore a persisted panel.
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
                if (msg.type !== 'open') return;
                const entry = this._indexEntry(msg.segmentId);
                // detail is a filename relative to docs/arrows/ (e.g. "marketing-site.md")
                const detailFile = entry?.detail ?? `${msg.segmentId}.md`;
                const docPath = path.join(this._workspaceRoot, 'docs', 'arrows', detailFile);
                void Promise.resolve(vscode.workspace.openTextDocument(docPath))
                    .then((doc) =>
                        vscode.window.showTextDocument(doc, vscode.ViewColumn.One),
                    )
                    .catch((err: unknown) => {
                        void vscode.window.showErrorMessage(
                            `LID Navigator: could not open ${docPath} — ${String(err)}`,
                        );
                    });
            },
            null,
            this._disposables,
        );

        // Watch index.yaml; re-post graph on any change.
        const watcher = vscode.workspace.createFileSystemWatcher(
            new vscode.RelativePattern(workspaceRoot, 'docs/arrows/index.yaml'),
        );
        const repost = () => this._postGraph();
        watcher.onDidChange(repost, null, this._disposables);
        watcher.onDidCreate(repost, null, this._disposables);
        this._disposables.push(watcher);

        // Delay first post so the webview script has time to initialise.
        setTimeout(() => this._postGraph(), 150);
    }

    // ── Private helpers ─────────────────────────────────────────────────────

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

    private _buildPayload(): GraphPayload {
        const arrows = this._loadIndex().arrows ?? {};
        const nodes: GraphNode[] = Object.entries(arrows).map(([id, entry]) => ({
            id,
            label: id,
            status: entry.status ?? 'UNMAPPED',
        }));

        // Use `blocks` edges only — `blockedBy` is the mirror image.
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

// ── Serializer ──────────────────────────────────────────────────────────────

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

// ── HTML builder ─────────────────────────────────────────────────────────────

function webviewOptions(extensionUri: vscode.Uri): vscode.WebviewOptions & vscode.WebviewPanelOptions {
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
    html, body {
      margin: 0; padding: 0;
      width: 100%; height: 100%;
      overflow: hidden;
      background: var(--vscode-editor-background, #1e1e1e);
      color: var(--vscode-editor-foreground, #d4d4d4);
    }
    #cy { width: 100%; height: 100%; }
    #empty {
      display: none;
      position: absolute; top: 50%; left: 50%;
      transform: translate(-50%, -50%);
      opacity: 0.5; font-family: sans-serif; font-size: 14px;
    }
  </style>
</head>
<body>
  <div id="cy"></div>
  <div id="empty">No segments found in docs/arrows/index.yaml</div>
  <script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
}
