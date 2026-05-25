// VS Code entry point for the LID extension.
//
// On activation we spawn the `lid-lsp` binary and connect it to the
// editor via `vscode-languageclient`. The binary is located by, in
// order of precedence:
//
//   1. The `lid.serverPath` setting (when non-empty)
//   2. The bundled binary at `<extension>/server/lid-lsp[.exe]`
//      (populated by the release pipeline once M4.37 lands)
//   3. The unqualified name `lid-lsp` — relies on the user having
//      `cargo install`ed it or otherwise put it on PATH.
//
// The trace channel is automatically wired by `vscode-languageclient`
// to the `lid.trace.server` setting declared in `package.json`.

import * as fs from 'fs';
import * as path from 'path';

import * as vscode from 'vscode';
import {
    Executable,
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
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
            { scheme: 'file', pattern: '**/docs/specs/**/*.md' },
            { scheme: 'file', pattern: '**/docs/llds/**/*.md' },
            { scheme: 'file', pattern: '**/docs/arrows/**/*.md' },
            { scheme: 'file', pattern: '**/docs/arrows/index.yaml' },
            { scheme: 'file', pattern: '**/docs/high-level-design.md' },
        ],
        outputChannelName: 'LID',
    };

    client = new LanguageClient('lid', 'LID', serverOptions, clientOptions);
    try {
        await client.start();
        console.log(`LID LSP client started (server: ${serverPath})`);
    } catch (err) {
        const message =
            err instanceof Error ? err.message : String(err);
        vscode.window.showErrorMessage(
            `LID: failed to start lid-lsp (\`${serverPath}\`): ${message}. ` +
                'Set `lid.serverPath` or install `lid-lsp` on PATH.',
        );
        client = undefined;
    }
}

export function deactivate(): Thenable<void> | undefined {
    return client?.stop();
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
