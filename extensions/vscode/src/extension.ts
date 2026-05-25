// VS Code entry point for the LID extension.
//
// Today the activate hook just logs that the extension started; the
// language-client bootstrap (which spawns `lid-lsp` and connects it
// to the editor) lands in the next commit.

import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext): void {
    console.log(`LID extension ${context.extension.packageJSON.version} activated`);
}

export function deactivate(): Thenable<void> | undefined {
    return undefined;
}
