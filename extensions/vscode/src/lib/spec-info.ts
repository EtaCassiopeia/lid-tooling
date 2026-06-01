// Pure file-system scan of docs/intent/ — no VS Code APIs, fully testable.

import * as fs from 'fs';
import * as path from 'path';

export interface SpecCounts {
    implemented: number;
    open: number;
    deferred: number;
}

export interface SpecItem {
    marker: 'done' | 'open' | 'deferred';
    id: string;
    text: string;
    line: number;
}

export interface SpecInfo {
    counts: SpecCounts;
    items: SpecItem[];
    specFile?: string;
    lldFile?: string;
    specPrefix?: string;
    decisionDocs?: string[];
}

const SPEC_RE = /^\s*-\s+\[([xX D])\]\s+\*\*([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)\*\*:\s*(.*)/;

export function buildSpecInfo(intentDir: string): Map<string, SpecInfo> {
    const result = new Map<string, SpecInfo>();

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
                if (ent.name === 'decisions') {
                    const segId = path.basename(dir);
                    const info = ensureEntry(segId);
                    try {
                        const decFiles = fs.readdirSync(fullPath, { withFileTypes: true });
                        for (const f of decFiles) {
                            if (f.isFile() && f.name.endsWith('.md')) {
                                if (!info.decisionDocs) info.decisionDocs = [];
                                info.decisionDocs.push(path.join(fullPath, f.name));
                            }
                        }
                        info.decisionDocs?.sort();
                    } catch { /* skip unreadable dir */ }
                } else {
                    walkDir(fullPath);
                }
            } else if (ent.isFile()) {
                if (ent.name.endsWith('-specs.md')) {
                    const segId = ent.name.slice(0, -'-specs.md'.length);
                    const info = ensureEntry(segId);
                    if (!info.specFile) info.specFile = fullPath;
                    try {
                        const content = fs.readFileSync(fullPath, 'utf8');
                        const lines = content.split('\n');
                        if (lines[0]?.trim() === '---') {
                            const closeIdx = lines.findIndex((l, i) => i > 0 && l.trim() === '---');
                            if (closeIdx > 0) {
                                for (let i = 1; i < closeIdx; i++) {
                                    const match = /^prefix:\s*(.+)$/.exec(lines[i]!);
                                    if (match) { info.specPrefix = match[1]!.trim(); break; }
                                }
                            }
                        }
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
                    } catch { /* skip unreadable file */ }
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
