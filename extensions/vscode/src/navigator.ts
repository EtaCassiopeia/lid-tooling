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
    children?: string[];
    parent?: string;
    next?: string;
    drift?: string;
    sampled?: string;
    audited?: string;
}

interface ArrowIndex {
    arrows?: Record<string, ArrowEntry>;
    taxonomy?: Record<string, string[]>;
}

interface SpecCounts {
    implemented: number;
    open: number;
    deferred: number;
}

interface SpecItem {
    marker: 'done' | 'open' | 'deferred';
    id: string;
    text: string;
    line: number;
}

interface SpecInfo {
    counts: SpecCounts;
    items: SpecItem[];
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
    specItems?: SpecItem[];
    specFile?: string;
    lldFile?: string;
    children?: string[];
    parent?: string;
}

interface GraphEdge {
    source: string;
    target: string;
    kind: 'blocks' | 'child';
}

interface GraphPayload {
    nodes: GraphNode[];
    edges: GraphEdge[];
    clusters: Record<string, string[]>;
}

type WebviewMessage =
    | { type: 'open'; segmentId: string }
    | { type: 'openFile'; path: string; line?: number }
    | { type: 'updateSpecStatus'; specFile: string; specId: string; line: number; newStatus: 'open' | 'implemented' | 'deferred' }
    | { type: 'addSpec'; specFile: string | undefined; segmentId: string; specId: string; text: string }
    | { type: 'updateSegmentStatus'; segmentId: string; newStatus: string }
    | { type: 'updateSegmentMeta'; segmentId: string; next: string; drift: string }
    | { type: 'addSegment'; segmentId: string; status: string; detail: string; blocks: string[]; children: string[] }
    | { type: 'removeConnection'; kind: 'blocks' | 'blockedBy' | 'children' | 'parent'; segmentId: string; target: string };

// ── NavigatorPanel ────────────────────────────────────────────────────────────

export class NavigatorPanel {
    public static currentPanel: NavigatorPanel | undefined;

    private readonly _panel: vscode.WebviewPanel;
    private readonly _workspaceRoot: string;
    private readonly _disposables: vscode.Disposable[] = [];
    private _refreshTimer: ReturnType<typeof setTimeout> | undefined;

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
                    this._openDocAtPath(msg.path, msg.line);
                } else if (msg.type === 'open') {
                    const entry = this._indexEntry(msg.segmentId);
                    const detailFile = entry?.detail ?? `${msg.segmentId}.md`;
                    this._openDocAtPath(
                        path.join(this._workspaceRoot, 'docs', 'arrows', detailFile),
                    );
                } else {
                    void this._handleMutation(msg);
                }
            },
            null,
            this._disposables,
        );

        // Re-post graph whenever index.yaml or any intent file changes on disk.
        const repost = () => this._scheduleRefresh();
        for (const glob of ['docs/arrows/index.yaml', 'docs/intent/**/*.md']) {
            const watcher = vscode.workspace.createFileSystemWatcher(
                new vscode.RelativePattern(workspaceRoot, glob),
            );
            watcher.onDidChange(repost, null, this._disposables);
            watcher.onDidCreate(repost, null, this._disposables);
            watcher.onDidDelete(repost, null, this._disposables);
            this._disposables.push(watcher);
        }

        setTimeout(() => this._postGraph(), 150);
    }

    // ── Private helpers ─────────────────────────────────────────────────────

    private _openDocAtPath(docPath: string, line?: number): void {
        void Promise.resolve(vscode.workspace.openTextDocument(docPath))
            .then((doc) => vscode.window.showTextDocument(doc, vscode.ViewColumn.One))
            .then((editor) => {
                if (line !== undefined && line > 0) {
                    const pos = new vscode.Position(line - 1, 0);
                    editor.revealRange(
                        new vscode.Range(pos, pos),
                        vscode.TextEditorRevealType.InCenter,
                    );
                    editor.selection = new vscode.Selection(pos, pos);
                }
            })
            .catch((err: unknown) => {
                void vscode.window.showErrorMessage(
                    `LID Navigator: could not open ${docPath} — ${String(err)}`,
                );
            });
    }

    private async _handleMutation(msg: WebviewMessage): Promise<void> {
        try {
            if (msg.type === 'updateSpecStatus') {
                await this._applySpecStatus(msg.specFile, msg.specId, msg.line, msg.newStatus);
                this._postMutationResult(true, `Spec ${msg.specId} marked as ${msg.newStatus}`);
            } else if (msg.type === 'addSpec') {
                const specFile = msg.specFile ?? path.join(
                    this._workspaceRoot, 'docs', 'intent', msg.segmentId, `${msg.segmentId}-specs.md`,
                );
                await this._appendSpec(specFile, msg.segmentId, msg.specId, msg.text);
                this._postMutationResult(true, `Spec ${msg.specId} added`);
            } else if (msg.type === 'updateSegmentStatus') {
                await this._updateIndexEntry(msg.segmentId, { status: msg.newStatus });
                this._postMutationResult(true, `Segment ${msg.segmentId} → ${msg.newStatus}`);
            } else if (msg.type === 'updateSegmentMeta') {
                await this._updateIndexEntry(msg.segmentId, {
                    next:  msg.next  || undefined,
                    drift: msg.drift || undefined,
                });
                this._postMutationResult(true, `Segment ${msg.segmentId} metadata saved`);
            } else if (msg.type === 'addSegment') {
                const entry: Partial<ArrowEntry> = { status: msg.status, detail: msg.detail };
                if (msg.blocks.length)   { entry.blocks   = msg.blocks; }
                if (msg.children.length) { entry.children = msg.children; }
                await this._updateIndexEntry(msg.segmentId, entry, true);
                for (const child of msg.children) {
                    await this._updateIndexEntry(child, { parent: msg.segmentId });
                }
                this._postMutationResult(true, `Segment ${msg.segmentId} added`);
            } else if (msg.type === 'removeConnection') {
                await this._removeConnection(msg.kind, msg.segmentId, msg.target);
                this._postMutationResult(true, 'Connection removed');
            }
        } catch (err: unknown) {
            this._postMutationResult(false, String(err));
        }
    }

    private _postMutationResult(ok: boolean, message: string): void {
        void this._panel.webview.postMessage({ type: ok ? 'mutationOk' : 'mutationError', message });
    }

    private async _applySpecStatus(
        specFile: string,
        specId: string,
        line: number,
        newStatus: 'open' | 'implemented' | 'deferred',
    ): Promise<void> {
        const uri = vscode.Uri.file(specFile);
        const doc = await vscode.workspace.openTextDocument(uri);
        const lineIdx = line - 1;
        const lineText = doc.lineAt(lineIdx).text;
        const newMarker = newStatus === 'implemented' ? 'x' : newStatus === 'deferred' ? 'D' : ' ';
        const newText = lineText.replace(/^(\s*-\s+\[)[xX D](\].*)$/, `$1${newMarker}$2`);
        if (newText === lineText) {
            throw new Error(`No spec marker found for ${specId} on line ${line}`);
        }
        const edit = new vscode.WorkspaceEdit();
        edit.replace(uri, doc.lineAt(lineIdx).range, newText);
        await vscode.workspace.applyEdit(edit);
        await doc.save(); // flush to disk so the file watcher triggers a graph refresh
    }

    private async _appendSpec(specFile: string, segmentId: string, specId: string, text: string): Promise<void> {
        const uri = vscode.Uri.file(specFile);
        const newLine = `- [ ] **${specId}**: ${text}\n`;

        let exists = true;
        try { await vscode.workspace.fs.stat(uri); }
        catch { exists = false; }

        if (exists) {
            const doc = await vscode.workspace.openTextDocument(uri);
            const end = doc.lineAt(doc.lineCount - 1).rangeIncludingLineBreak.end;
            const suffix = doc.getText().endsWith('\n') ? '' : '\n';
            const edit = new vscode.WorkspaceEdit();
            edit.insert(uri, end, suffix + newLine);
            await vscode.workspace.applyEdit(edit);
            await doc.save(); // flush to disk so the file watcher triggers a graph refresh
        } else {
            await vscode.workspace.fs.createDirectory(vscode.Uri.file(path.dirname(specFile)));
            await vscode.workspace.fs.writeFile(uri, Buffer.from(`# ${segmentId} specs\n\n${newLine}`));
        }
    }

    private async _updateIndexEntry(
        segmentId: string,
        changes: Partial<ArrowEntry>,
        create = false,
    ): Promise<void> {
        const indexPath = path.join(this._workspaceRoot, 'docs', 'arrows', 'index.yaml');
        const index = this._loadIndex();
        if (!index.arrows) { index.arrows = {}; }
        if (!create && !index.arrows[segmentId]) {
            throw new Error(`Segment '${segmentId}' not found in index.yaml`);
        }
        const merged = { ...(index.arrows[segmentId] ?? {}), ...changes };
        for (const key of Object.keys(merged)) {
            if ((merged as Record<string, unknown>)[key] === undefined) {
                delete (merged as Record<string, unknown>)[key];
            }
        }
        index.arrows[segmentId] = merged;
        const yamlStr = yaml.dump(index, { lineWidth: -1 });
        await vscode.workspace.fs.writeFile(vscode.Uri.file(indexPath), Buffer.from(yamlStr));
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

    private async _removeConnection(
        kind: 'blocks' | 'blockedBy' | 'children' | 'parent',
        segmentId: string,
        target: string,
    ): Promise<void> {
        if (kind === 'blocks') {
            const entry = this._indexEntry(segmentId);
            const next = (entry?.blocks ?? []).filter(b => b !== target);
            await this._updateIndexEntry(segmentId, { blocks: next.length ? next : undefined });
        } else if (kind === 'blockedBy') {
            // segmentId is blocked by target → remove segmentId from target.blocks[]
            const entry = this._indexEntry(target);
            const next = (entry?.blocks ?? []).filter(b => b !== segmentId);
            await this._updateIndexEntry(target, { blocks: next.length ? next : undefined });
        } else if (kind === 'children') {
            const entry = this._indexEntry(segmentId);
            const next = (entry?.children ?? []).filter(c => c !== target);
            await this._updateIndexEntry(segmentId, { children: next.length ? next : undefined });
            const child = this._indexEntry(target);
            if (child?.parent === segmentId) {
                await this._updateIndexEntry(target, { parent: undefined });
            }
        } else if (kind === 'parent') {
            // clear segmentId from target.children[]
            const parentEntry = this._indexEntry(target);
            const next = (parentEntry?.children ?? []).filter(c => c !== segmentId);
            await this._updateIndexEntry(target, { children: next.length ? next : undefined });
            const entry = this._indexEntry(segmentId);
            if (entry?.parent === target) {
                await this._updateIndexEntry(segmentId, { parent: undefined });
            }
        }
    }

    private _buildSpecInfo(): Map<string, SpecInfo> {
        const intentDir = path.join(this._workspaceRoot, 'docs', 'intent');
        const result = new Map<string, SpecInfo>();
        const SPEC_RE = /^\s*-\s+\[([xX D])\]\s+\*\*([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)\*\*:\s*(.*)/;

        const ensureEntry = (segId: string): SpecInfo => {
            if (!result.has(segId)) {
                result.set(segId, { counts: { implemented: 0, open: 0, deferred: 0 }, items: [] });
            }
            return result.get(segId)!;
        };

        const walkDir = (dir: string) => {
            let entries: fs.Dirent[];
            try { entries = fs.readdirSync(dir, { withFileTypes: true }); }
            catch { return; }
            for (const ent of entries) {
                const fullPath = path.join(dir, ent.name);
                if (ent.isDirectory()) {
                    walkDir(fullPath);
                } else if (ent.isFile()) {
                    if (ent.name.endsWith('-specs.md')) {
                        const segId = ent.name.slice(0, -'-specs.md'.length);
                        const info = ensureEntry(segId);
                        if (!info.specFile) info.specFile = fullPath;
                        try {
                            const lines = fs.readFileSync(fullPath, 'utf8').split('\n');
                            lines.forEach((rawLine, idx) => {
                                const m = SPEC_RE.exec(rawLine);
                                if (!m) return;
                                const ch = m[1]!;
                                const id = m[2]!;
                                const text = (m[3] ?? '').trim();
                                const lineNo = idx + 1;
                                if (ch === 'x' || ch === 'X') {
                                    info.counts.implemented++;
                                    info.items.push({ marker: 'done', id, text, line: lineNo });
                                } else if (ch === ' ') {
                                    info.counts.open++;
                                    info.items.push({ marker: 'open', id, text, line: lineNo });
                                } else {
                                    info.counts.deferred++;
                                    info.items.push({ marker: 'deferred', id, text, line: lineNo });
                                }
                            });
                        } catch {
                            // skip unreadable file
                        }
                    } else if (ent.name.endsWith('-design.md')) {
                        const segId = ent.name.slice(0, -'-design.md'.length);
                        const info = ensureEntry(segId);
                        if (!info.lldFile) info.lldFile = fullPath;
                    }
                }
            }
        };

        walkDir(intentDir);
        return result;
    }

    private _buildPayload(): GraphPayload {
        const index = this._loadIndex();
        const arrows = index.arrows ?? {};
        const clusters: Record<string, string[]> = {};
        for (const [name, ids] of Object.entries(index.taxonomy ?? {})) {
            clusters[name] = ids;
        }
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
                specItems: info?.items,
                specFile: info?.specFile,
                lldFile: info?.lldFile,
                children: entry.children,
                parent: entry.parent,
            };
        });

        const edges: GraphEdge[] = [];
        const seen = new Set<string>();
        for (const [id, entry] of Object.entries(arrows)) {
            for (const target of entry.blocks ?? []) {
                const key = `blocks\x00${id}\x00${target}`;
                if (!seen.has(key) && target in arrows) {
                    seen.add(key);
                    edges.push({ source: id, target, kind: 'blocks' });
                }
            }
            for (const child of entry.children ?? []) {
                const key = `child\x00${id}\x00${child}`;
                if (!seen.has(key) && child in arrows) {
                    seen.add(key);
                    edges.push({ source: id, target: child, kind: 'child' });
                }
            }
        }

        return { nodes, edges, clusters };
    }

    // Debounce rapid file-system events (e.g. agent writing multiple files, mid-keystroke saves).
    // The 400 ms window lets a flurry of saves settle before we re-read disk.
    private _scheduleRefresh(): void {
        if (this._refreshTimer !== undefined) {
            clearTimeout(this._refreshTimer);
        }
        this._refreshTimer = setTimeout(() => {
            this._refreshTimer = undefined;
            this._postGraph();
        }, 400);
    }

    private _postGraph(): void {
        let payload: GraphPayload;
        try {
            payload = this._buildPayload();
        } catch {
            // Transient filesystem state (half-written file, missing dir).
            // Keep the webview unchanged until the next stable read.
            return;
        }
        void this._panel.webview.postMessage({ type: 'graph', payload });
    }

    // ── Lifecycle ───────────────────────────────────────────────────────────

    public dispose(): void {
        NavigatorPanel.currentPanel = undefined;
        if (this._refreshTimer !== undefined) {
            clearTimeout(this._refreshTimer);
            this._refreshTimer = undefined;
        }
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
    /* ── Rift Design Tokens ── */
    :root {
      --bg-primary:    #0f0f1a;
      --bg-secondary:  #1a1b2e;
      --bg-card:       #1e2235;
      --bg-card-hover: #252a40;
      --accent-coral:  #E07A5F;
      --accent-green:  #81B29A;
      --accent-blue:   #6366F1;
      --accent-teal:   #14B8A6;
      --accent-yellow: #F59E0B;
      --accent-red:    #EF4444;
      --text-primary:  #FFFFFF;
      --text-secondary:#9CA3AF;
      --text-muted:    #6B7280;
      --border:        #2D3348;
      --r-sm: 6px; --r-md: 12px; --r-full: 9999px;
      --font: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
      --mono: 'SF Mono', 'Consolas', 'JetBrains Mono', monospace;
    }
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    html, body {
      width: 100%; height: 100%; overflow: hidden;
      background: var(--bg-primary);
      color: var(--text-secondary);
      font-family: var(--font); font-size: 12px;
      display: flex; flex-direction: column;
    }
    /* ── Toolbar ── */
    #toolbar {
      height: 38px; flex-shrink: 0;
      display: flex; align-items: center; gap: 8px; padding: 0 12px;
      background: var(--bg-secondary);
      border-bottom: 1px solid var(--border);
    }
    #toolbar button {
      background: rgba(224,122,95,.12);
      color: var(--accent-coral);
      border: 1px solid rgba(224,122,95,.3);
      border-radius: var(--r-sm); padding: 3px 10px;
      cursor: pointer; font-size: 11px; font-family: var(--font); font-weight: 500;
      transition: background .15s;
    }
    #toolbar button:hover { background: rgba(224,122,95,.22); }
    #btn-clear {
      background: transparent; border: none !important;
      color: var(--text-muted); padding: 0 4px; font-size: 14px;
    }
    #btn-clear:hover { color: var(--text-primary); background: transparent !important; }
    #toolbar label {
      display: flex; align-items: center; gap: 4px; font-size: 11px;
      color: var(--text-muted);
    }
    #toolbar select, #toolbar input[type=text] {
      background: var(--bg-card);
      color: var(--text-secondary);
      border: 1px solid var(--border);
      border-radius: var(--r-sm); padding: 2px 7px; font-size: 11px;
      font-family: var(--font); outline: none;
      transition: border-color .15s;
    }
    #toolbar select:focus, #toolbar input[type=text]:focus { border-color: var(--accent-coral); }
    #search { width: 140px; }
    .tb-sep { width: 1px; height: 16px; background: var(--border); }
    /* ── Main + graph canvas ── */
    #main { flex: 1; display: flex; overflow: hidden; min-height: 0; }
    #cy {
      flex: 1; min-width: 0;
      background:
        linear-gradient(rgba(255,255,255,.025) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255,255,255,.025) 1px, transparent 1px),
        var(--bg-primary);
      background-size: 40px 40px;
    }
    /* ── Sidebar ── */
    #panel {
      width: 268px; flex-shrink: 0;
      display: none; flex-direction: column;
      border-left: 1px solid var(--border);
      background: var(--bg-secondary);
    }
    #panel.open { display: flex; }
    .ph {
      display: flex; align-items: center; gap: 6px;
      padding: 9px 12px; flex-shrink: 0;
      border-bottom: 1px solid var(--border);
    }
    .ph-title {
      overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
      font-weight: 600; font-size: 13px; color: var(--text-primary); flex: 1;
    }
    .ph-close { cursor: pointer; color: var(--text-muted); padding: 0 2px; font-size: 14px; flex-shrink: 0; }
    .ph-close:hover { color: var(--text-primary); }
    .pb { overflow-y: auto; padding: 10px; flex: 1; }
    /* ── Status badges ── */
    .badge {
      display: inline-flex; align-items: center;
      padding: 3px 10px; border-radius: var(--r-full);
      font-size: 10px; font-weight: 600;
      text-transform: uppercase; letter-spacing: .05em;
      margin-bottom: 10px;
    }
    .badge-UNMAPPED { background: rgba(107,114,128,.15); border: 1px solid rgba(107,114,128,.3); color: #9CA3AF; }
    .badge-MAPPED   { background: rgba(99,102,241,.15);  border: 1px solid rgba(99,102,241,.3);  color: #6366F1; }
    .badge-AUDITED  { background: rgba(245,158,11,.15);  border: 1px solid rgba(245,158,11,.3);  color: #F59E0B; }
    .badge-OK       { background: rgba(129,178,154,.15); border: 1px solid rgba(129,178,154,.3); color: #81B29A; }
    .badge-MERGED   { background: rgba(20,184,166,.15);  border: 1px solid rgba(20,184,166,.3);  color: #14B8A6; }
    /* ── Panel sections (card-style) ── */
    .sec {
      margin-top: 8px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--r-sm);
      padding: 8px 10px;
    }
    .sec-title {
      font-size: 9px; text-transform: uppercase;
      letter-spacing: .1em; color: var(--text-muted);
      font-weight: 500; margin-bottom: 5px;
    }
    .prog-wrap { height: 4px; border-radius: 2px; background: rgba(255,255,255,.06); overflow: hidden; margin: 4px 0; }
    .prog-fill { height: 100%; border-radius: 2px; }
    .spec-row { display: flex; gap: 10px; font-size: 10px; color: var(--text-muted); margin-top: 3px; }
    .meta { display: grid; grid-template-columns: auto 1fr; gap: 2px 8px; font-size: 10px; }
    .meta-k { color: var(--text-muted); }
    /* Action buttons */
    .btn-open {
      display: block; width: 100%; margin-top: 5px;
      padding: 5px 10px; text-align: left;
      background: rgba(224,122,95,.08);
      color: var(--accent-coral);
      border: 1px solid rgba(224,122,95,.3);
      border-radius: var(--r-sm); cursor: pointer; font-size: 11px;
      font-family: var(--font); transition: background .15s;
    }
    .btn-open:hover { background: rgba(224,122,95,.18); }
    .btn-open:first-child { margin-top: 0; }
    .prose {
      font-size: 11px; line-height: 1.6; color: var(--text-secondary);
      white-space: pre-wrap; word-break: break-word;
    }
    /* ── Dependency chips ── */
    .chips { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 4px; }
    .chip {
      padding: 2px 9px; border-radius: var(--r-full); font-size: 10px; cursor: pointer;
      background: rgba(20,184,166,.1);
      color: var(--accent-teal);
      border: 1px solid rgba(20,184,166,.28);
      max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
      font-weight: 500; font-family: var(--font); transition: background .15s;
    }
    .chip:hover { background: rgba(20,184,166,.22); }
    /* ── Spec list ── */
    .sec-title-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px; }
    .spec-toggle { background: none; border: none; font-size: 10px; color: var(--text-muted); cursor: pointer; padding: 0; }
    .spec-toggle:hover { color: var(--text-primary); }
    .spec-list { margin-top: 6px; max-height: 200px; overflow-y: auto; border-top: 1px solid var(--border); padding-top: 4px; }
    .si { display: flex; gap: 6px; padding: 3px 0; font-size: 10px; align-items: flex-start; border-bottom: 1px solid rgba(45,51,72,.5); }
    .si:last-child { border-bottom: none; }
    .si-id {
      font-size: 9px; font-family: var(--mono); color: var(--accent-teal);
      background: none; border: none; padding: 0; cursor: pointer;
      text-decoration: none; white-space: nowrap; flex-shrink: 0; font-weight: 500;
    }
    .si-id:hover { text-decoration: underline; }
    .si-m { flex-shrink: 0; width: 14px; text-align: center; font-size: 11px; }
    .si-done { color: var(--accent-green); }
    .si-open { color: var(--accent-yellow); }
    .si-def  { color: var(--text-muted); }
    .si-t { color: var(--text-secondary); word-break: break-word; line-height: 1.5; }
    /* ── Header buttons (Focus / Edit) ── */
    .btn-focus, #btn-edit {
      background: transparent;
      border: 1px solid var(--border); color: var(--text-muted);
      border-radius: var(--r-sm); padding: 2px 8px;
      font-size: 10px; cursor: pointer; flex-shrink: 0;
      font-family: var(--font); font-weight: 500; transition: all .15s;
    }
    .btn-focus:hover:not(.active), #btn-edit:hover:not(.active) { color: var(--text-primary); border-color: var(--text-muted); }
    .btn-focus.active, #btn-edit.active { background: var(--accent-coral); color: #fff; border-color: transparent; }
    /* ── Mutation feedback banner ── */
    #mut-banner { display: none; padding: 6px 12px; font-size: 11px; flex-shrink: 0; border-bottom: 1px solid var(--border); }
    #mut-banner.ok  { background: rgba(129,178,154,.15); border-bottom-color: rgba(129,178,154,.3); color: var(--accent-green); }
    #mut-banner.err { background: rgba(239,68,68,.15);   border-bottom-color: rgba(239,68,68,.3);   color: #F87171; }
    /* ── Edit-mode form elements ── */
    .edit-select, .edit-textarea, .edit-input {
      width: 100%;
      background: var(--bg-primary); color: var(--text-primary);
      border: 1px solid var(--border);
      border-radius: var(--r-sm); padding: 4px 8px; font-size: 11px; outline: none;
      font-family: var(--font); box-sizing: border-box; transition: border-color .15s;
    }
    .edit-select:focus, .edit-textarea:focus, .edit-input:focus { border-color: var(--accent-coral); }
    .edit-textarea { resize: vertical; min-height: 60px; }
    .btn-save {
      margin-top: 5px; padding: 4px 14px;
      background: var(--accent-coral); color: #fff;
      border: none; border-radius: var(--r-sm);
      cursor: pointer; font-size: 11px; font-family: var(--font); font-weight: 500;
      transition: background .15s;
    }
    .btn-save:hover { background: #c96a52; }
    .si-toggle { background: none; border: none; cursor: pointer; padding: 0; display: flex; align-items: center; flex-shrink: 0; line-height: 1; transition: opacity .15s; }
    .si-toggle:hover { opacity: 0.7; }
    .add-spec-btn {
      display: block; width: 100%; margin-top: 6px;
      background: transparent; border: 1px dashed rgba(45,51,72,.9);
      color: var(--text-muted); border-radius: var(--r-sm);
      padding: 4px 8px; font-size: 10px; cursor: pointer; text-align: left;
      font-family: var(--font); transition: all .15s;
    }
    .add-spec-btn:hover { border-color: var(--accent-coral); color: var(--accent-coral); }
    .add-spec-form { margin-top: 6px; display: none; }
    .add-spec-form.open { display: block; }
    .add-spec-row { display: flex; gap: 4px; margin-top: 4px; }
    .add-spec-row .edit-input { flex: 1; }
    .add-spec-err { font-size: 10px; color: #F87171; margin-top: 3px; min-height: 14px; }
    /* ── Removable chips ── */
    .chip-rm { display: inline-flex; align-items: center; }
    .chip-x {
      display: none; background: none; border: none; padding: 0 0 0 4px;
      color: #F87171; cursor: pointer; font-size: 13px; font-weight: 700;
      line-height: 1; flex-shrink: 0;
    }
    .chip-rm:hover .chip-x { display: inline; }
    .picker-chips { display: flex; flex-wrap: wrap; gap: 4px; min-height: 20px; margin-bottom: 4px; }
    /* ── Add-segment overlay ── */
    #add-seg-overlay {
      display: none; position: fixed; inset: 0;
      background: rgba(0,0,0,.72); z-index: 2000;
      align-items: flex-start; justify-content: center; padding-top: 60px;
      backdrop-filter: blur(4px);
    }
    #add-seg-overlay.open { display: flex; }
    #add-seg-box {
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--r-md); padding: 20px; width: 380px;
      box-shadow: 0 20px 40px rgba(0,0,0,.6);
    }
    #add-seg-box h3 { font-size: 14px; margin-bottom: 16px; color: var(--text-primary); font-weight: 600; }
    .seg-field { margin-bottom: 10px; }
    .seg-field label {
      display: block; font-size: 9px; text-transform: uppercase;
      letter-spacing: .1em; color: var(--text-muted); margin-bottom: 4px; font-weight: 500;
    }
    .seg-field-row { display: flex; gap: 8px; margin-top: 16px; justify-content: flex-end; }
    .btn-cancel {
      background: transparent; border: 1px solid var(--border);
      color: var(--text-secondary); border-radius: var(--r-sm);
      padding: 5px 14px; cursor: pointer; font-size: 11px;
      font-family: var(--font); font-weight: 500; transition: all .15s;
    }
    .btn-cancel:hover { border-color: var(--text-muted); color: var(--text-primary); }
    #seg-add-err { font-size: 10px; color: #F87171; margin-top: 6px; min-height: 14px; }
    /* ── Tooltip ── */
    #tooltip {
      position: fixed; pointer-events: none; max-width: 300px;
      background: var(--bg-card); border: 1px solid var(--border);
      border-radius: var(--r-sm); padding: 10px 12px;
      font-size: 11px; z-index: 999; display: none; line-height: 1.5;
      box-shadow: 0 8px 24px rgba(0,0,0,.5);
    }
    .tt-title { font-weight: 600; margin-bottom: 4px; color: var(--text-primary); }
    .tt-row { color: var(--text-muted); margin-top: 2px; }
    .tt-sep { border: none; border-top: 1px solid var(--border); margin: 6px 0; }
    /* ── Context menu ── */
    #ctxmenu {
      position: fixed;
      background: var(--bg-card); border: 1px solid var(--border);
      border-radius: var(--r-sm); padding: 4px 0;
      font-size: 12px; z-index: 1000; display: none;
      min-width: 175px; box-shadow: 0 8px 24px rgba(0,0,0,.6);
    }
    .ctx-item { padding: 6px 14px; cursor: pointer; color: var(--text-secondary); }
    .ctx-item:hover { background: rgba(99,102,241,.12); color: var(--text-primary); }
    .ctx-item.hidden { display: none; }
    .ctx-sep { border-top: 1px solid var(--border); margin: 3px 0; }
    /* ── Legend ── */
    #legend {
      position: fixed; bottom: 12px; left: 12px;
      background: var(--bg-card); border: 1px solid var(--border);
      border-radius: var(--r-sm); padding: 8px 12px;
      font-size: 10px; pointer-events: none; line-height: 1.9;
      box-shadow: 0 4px 12px rgba(0,0,0,.4);
    }
    .leg-row { display: flex; align-items: center; gap: 5px; flex-wrap: wrap; color: var(--text-muted); }
    .sw { display: inline-block; width: 9px; height: 9px; border-radius: 50%; vertical-align: middle; }
    /* ── Empty state ── */
    #empty {
      display: none; position: fixed; top: 50%; left: 50%;
      transform: translate(-50%,-50%);
      color: var(--text-muted); font-size: 13px;
    }
  </style>
</head>
<body>
  <div id="toolbar">
    <button id="btn-fit">Fit</button>
    <button id="btn-add-seg">+ Segment</button>
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
    <label>Cluster
      <select id="filter-cluster">
        <option value="">All</option>
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
        <button id="btn-edit">✎ Edit</button>
        <button class="btn-focus" id="btn-focus">Focus</button>
        <span class="ph-close" id="panel-close">✕</span>
      </div>
      <div id="mut-banner"></div>
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
      <span class="sw" style="background:#6B7280"></span>UNMAPPED&nbsp;
      <span class="sw" style="background:#6366F1"></span>MAPPED&nbsp;
      <span class="sw" style="background:#F59E0B"></span>AUDITED&nbsp;
      <span class="sw" style="background:#81B29A"></span>OK&nbsp;
      <span class="sw" style="background:#14B8A6"></span>MERGED
    </div>
    <div class="leg-row" style="margin-top:3px">
      <span class="sw" style="background:transparent;border:2px solid #F59E0B;border-radius:3px"></span>has drift &nbsp;
      <span class="sw" style="background:transparent;border:2px solid #E07A5F;border-radius:3px"></span>has next
    </div>
  </div>

  <div id="empty">No segments found in docs/arrows/index.yaml</div>

  <div id="add-seg-overlay">
    <div id="add-seg-box">
      <h3>Add Segment</h3>
      <div class="seg-field">
        <label>Segment ID</label>
        <input class="edit-input" id="seg-id-input" placeholder="billing">
      </div>
      <div class="seg-field">
        <label>Status</label>
        <select class="edit-select" id="seg-status-select">
          <option value="UNMAPPED">UNMAPPED</option>
          <option value="MAPPED">MAPPED</option>
          <option value="AUDITED">AUDITED</option>
          <option value="OK">OK</option>
          <option value="MERGED">MERGED</option>
        </select>
      </div>
      <div class="seg-field">
        <label>Detail path (relative to docs/arrows/)</label>
        <input class="edit-input" id="seg-detail-input" placeholder="billing/core.md">
      </div>
      <div class="seg-field">
        <label>Blocks (optional)</label>
        <div class="picker-chips" id="seg-blocks-chips"></div>
        <select class="edit-select" id="seg-blocks-select">
          <option value="">— add segment —</option>
        </select>
      </div>
      <div class="seg-field">
        <label>Children (optional)</label>
        <div class="picker-chips" id="seg-children-chips"></div>
        <select class="edit-select" id="seg-children-select">
          <option value="">— add segment —</option>
        </select>
      </div>
      <div id="seg-add-err"></div>
      <div class="seg-field-row">
        <button class="btn-cancel" id="btn-seg-cancel">Cancel</button>
        <button class="btn-save" id="btn-seg-submit">Add</button>
      </div>
    </div>
  </div>

  <script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
}
