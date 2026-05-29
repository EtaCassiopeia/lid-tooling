// VS Code entry point for the LID extension.
//
// On activation we spawn the `lid-lsp` binary, connect it to the
// editor via `vscode-languageclient`, install a status-bar indicator
// for server state, and register a couple of commands for the user
// to interact with the server.

import * as fs from 'fs';
import * as path from 'path';

import * as vscode from 'vscode';
import {
    Executable,
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    State,
    TransportKind,
} from 'vscode-languageclient/node';

import { NavigatorPanel, NavigatorPanelSerializer } from './navigator';

const COMMAND_RESTART_SERVER = 'lid.restartServer';
const COMMAND_SHOW_OUTPUT = 'lid.showOutputChannel';
const COMMAND_SHOW_NAVIGATOR = 'lid.showIntentNavigator';

let client: LanguageClient | undefined;
let statusBarItem: vscode.StatusBarItem | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    statusBarItem = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right,
        100,
    );
    statusBarItem.command = COMMAND_SHOW_OUTPUT;
    statusBarItem.tooltip = 'Click to show the LID output channel';
    statusBarItem.text = '$(sync~spin) LID: starting…';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    context.subscriptions.push(
        vscode.commands.registerCommand(COMMAND_SHOW_OUTPUT, () => {
            client?.outputChannel.show(true);
        }),
        vscode.commands.registerCommand(COMMAND_RESTART_SERVER, async () => {
            if (client === undefined) {
                vscode.window.showInformationMessage(
                    'LID: server is not running; reload the window to start it.',
                );
                return;
            }
            setStatusStarting();
            try {
                await client.restart();
                setStatusReady();
            } catch (err) {
                setStatusError(err);
            }
        }),
        vscode.commands.registerCommand(COMMAND_SHOW_NAVIGATOR, () => {
            // Prefer the workspace folder that contains the active editor so
            // the correct repo is shown in multi-root workspaces.
            const activeUri = vscode.window.activeTextEditor?.document.uri;
            const folder = activeUri
                ? vscode.workspace.getWorkspaceFolder(activeUri)
                : vscode.workspace.workspaceFolders?.[0];
            const workspaceRoot = folder?.uri.fsPath;
            if (!workspaceRoot) {
                vscode.window.showWarningMessage(
                    'LID: open a workspace folder first.',
                );
                return;
            }
            NavigatorPanel.createOrShow(context.extensionUri, workspaceRoot);
        }),
    );

    vscode.window.registerWebviewPanelSerializer(
        'lid.intentNavigator',
        new NavigatorPanelSerializer(context.extensionUri),
    );

    await startServer(context);
}

export function deactivate(): Thenable<void> | undefined {
    statusBarItem?.dispose();
    statusBarItem = undefined;
    return client?.stop();
}

async function startServer(context: vscode.ExtensionContext): Promise<void> {
    const serverPath = resolveServerPath(context);
    const executable: Executable = {
        command: serverPath,
        transport: TransportKind.stdio,
    };
    const serverOptions: ServerOptions = {
        run: executable,
        debug: executable,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            // Source-code languages that conventionally carry `@spec`
            // citations. The LSP itself ignores languages where no
            // citations exist, so the over-broad list is fine.
            { scheme: 'file', language: 'rust' },
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'typescriptreact' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'javascriptreact' },
            { scheme: 'file', language: 'python' },
            { scheme: 'file', language: 'go' },
            { scheme: 'file', language: 'java' },
            { scheme: 'file', language: 'scala' },
            { scheme: 'file', language: 'csharp' },
            { scheme: 'file', language: 'ruby' },
            { scheme: 'file', language: 'cpp' },
            { scheme: 'file', language: 'c' },
            // LID artifact files.
            { scheme: 'file', pattern: '**/docs/intent/**/*.md' },
            { scheme: 'file', pattern: '**/docs/arrows/**/*.md' },
            { scheme: 'file', pattern: '**/docs/arrows/index.yaml' },
            { scheme: 'file', pattern: '**/docs/high-level-design.md' },
        ],
        outputChannelName: 'LID',
    };

    client = new LanguageClient('lid', 'LID', serverOptions, clientOptions);
    client.onDidChangeState((event) => {
        switch (event.newState) {
            case State.Running:
                setStatusReady();
                break;
            case State.Starting:
                setStatusStarting();
                break;
            case State.Stopped:
                setStatusStopped();
                break;
        }
    });

    try {
        await client.start();
    } catch (err) {
        setStatusError(err);
        vscode.window.showErrorMessage(
            `LID: failed to start lid-lsp (\`${serverPath}\`): ${formatError(err)}. ` +
                'Set `lid.serverPath` or install `lid-lsp` on PATH.',
        );
        client = undefined;
    }
}

function resolveServerPath(context: vscode.ExtensionContext): string {
    const configured = vscode.workspace
        .getConfiguration('lid')
        .get<string>('serverPath');
    if (configured !== undefined && configured.trim() !== '') {
        return configured.trim();
    }

    const exe = process.platform === 'win32' ? 'lid-lsp.exe' : 'lid-lsp';
    const bundled = path.join(context.extensionPath, 'server', exe);
    if (fs.existsSync(bundled)) {
        return bundled;
    }

    // Last resort: assume the user installed `lid-lsp` on PATH. If
    // that's wrong, the `client.start()` call throws and we surface
    // a clear error message above.
    return 'lid-lsp';
}

function setStatusStarting(): void {
    if (statusBarItem !== undefined) {
        statusBarItem.text = '$(sync~spin) LID: starting…';
        statusBarItem.backgroundColor = undefined;
    }
}

function setStatusReady(): void {
    if (statusBarItem !== undefined) {
        statusBarItem.text = '$(check) LID';
        statusBarItem.backgroundColor = undefined;
    }
}

function setStatusStopped(): void {
    if (statusBarItem !== undefined) {
        statusBarItem.text = '$(circle-slash) LID: stopped';
        statusBarItem.backgroundColor = new vscode.ThemeColor(
            'statusBarItem.warningBackground',
        );
    }
}

function setStatusError(err: unknown): void {
    if (statusBarItem !== undefined) {
        statusBarItem.text = '$(error) LID: error';
        statusBarItem.backgroundColor = new vscode.ThemeColor(
            'statusBarItem.errorBackground',
        );
        statusBarItem.tooltip = `LID: ${formatError(err)}. Click to view output.`;
    }
}

function formatError(err: unknown): string {
    if (err instanceof Error) {
        return err.message;
    }
    return String(err);
}
