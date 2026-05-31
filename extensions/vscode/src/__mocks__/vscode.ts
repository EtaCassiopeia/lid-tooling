// Minimal VS Code API stub for Jest unit tests.
// Only exports used by the modules under test need to be present.
export const Uri = {
    file: (p: string) => ({ fsPath: p }),
    joinPath: (base: { fsPath: string }, ...parts: string[]) => ({
        fsPath: [base.fsPath, ...parts].join('/'),
    }),
};
export const workspace = {
    fs: {
        createDirectory: async () => undefined,
        writeFile: async () => undefined,
        stat: async () => ({}),
    },
    workspaceFolders: undefined,
    openTextDocument: async () => ({}),
    createFileSystemWatcher: () => ({
        onDidChange: () => ({ dispose: () => undefined }),
        onDidCreate: () => ({ dispose: () => undefined }),
        onDidDelete: () => ({ dispose: () => undefined }),
        dispose: () => undefined,
    }),
};
export const window = {
    showErrorMessage: async () => undefined,
    showWarningMessage: async () => undefined,
    showTextDocument: async () => ({}),
    activeTextEditor: undefined,
    createWebviewPanel: () => ({}),
};
export const ViewColumn = { One: 1, Beside: 2 };
export const Position = class { constructor(public line: number, public character: number) {} };
export const Range = class { constructor(public start: unknown, public end: unknown) {} };
export const Selection = class { constructor(public anchor: unknown, public active: unknown) {} };
export const TextEditorRevealType = { InCenter: 2 };
export const RelativePattern = class { constructor(public base: unknown, public pattern: string) {} };
