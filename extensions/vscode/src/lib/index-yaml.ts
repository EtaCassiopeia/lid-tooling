// Pure helpers for reading and writing docs/arrows/index.yaml.
//
// All functions are side-effect-free (string-in / string-out or plain object
// out) so they can be unit-tested in Node.js without a VS Code environment.

export interface ArrowEntry {
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

/**
 * Serialise a new segment entry as indented YAML lines (no trailing newline).
 * Arrays are kept inline (`[a, b]`) to match the usual index.yaml style.
 */
export function buildSegmentBlock(segmentId: string, entry: Partial<ArrowEntry>): string {
    const lines: string[] = [`  ${segmentId}:`];
    const FIELD_ORDER: ReadonlyArray<keyof ArrowEntry> = [
        'status', 'detail', 'parent', 'blocks', 'children',
        'sampled', 'audited', 'next', 'drift',
    ];
    for (const k of FIELD_ORDER) {
        const v = entry[k];
        if (v === undefined || v === null) { continue; }
        if (Array.isArray(v)) {
            if (v.length === 0) { continue; }
            lines.push(`    ${k}: [${v.join(', ')}]`);
        } else if (typeof v === 'string') {
            const needsQuote = /[:#[\]{}|>&*!,?@`]/.test(v) || v.trim() !== v;
            lines.push(`    ${k}: ${needsQuote ? JSON.stringify(v) : v}`);
        } else {
            lines.push(`    ${k}: ${String(v)}`);
        }
    }
    return lines.join('\n');
}

/**
 * Insert `block` (a multi-line segment entry, no trailing newline) into the
 * `arrows:` section of `content`, immediately before the first top-level key
 * that follows the section.  Appends when `arrows:` is the last section.
 */
export function insertIntoArrows(content: string, block: string): string {
    const lines = content.split('\n');
    let insertAt = lines.length;
    let inArrows = false;
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i]!;
        if (/^arrows:/.test(line)) { inArrows = true; continue; }
        if (inArrows && line.length > 0 && !/^\s/.test(line)) {
            insertAt = i;
            break;
        }
    }
    return [...lines.slice(0, insertAt), block, ...lines.slice(insertAt)].join('\n');
}

/**
 * Add or replace a single scalar field on an existing segment without touching
 * any other part of the file.  If the field already exists it is replaced in
 * place; if it is absent it is appended after the last line of that segment.
 * Returns the content unchanged when `segmentId` is not found.
 */
export function patchSegmentField(
    content: string,
    segmentId: string,
    field: string,
    value: string,
): string {
    const lines = content.split('\n');
    const headerRe = new RegExp(`^  ${segmentId}:\\s*$`);
    let headerIdx = -1;
    for (let i = 0; i < lines.length; i++) {
        if (headerRe.test(lines[i]!)) { headerIdx = i; break; }
    }
    if (headerIdx === -1) { return content; }

    let end = headerIdx + 1;
    while (end < lines.length) {
        const l = lines[end]!;
        if (l.length > 0 && !/^    /.test(l)) { break; }
        end++;
    }

    const fieldRe = new RegExp(`^    ${field}:`);
    const newLine = `    ${field}: ${value}`;
    const existingIdx = lines.findIndex((l, i) => i > headerIdx && i < end && fieldRe.test(l));
    if (existingIdx !== -1) {
        lines[existingIdx] = newLine;
    } else {
        lines.splice(end, 0, newLine);
    }
    return lines.join('\n');
}

export interface ScaffoldContents {
    specsPath: string;
    specsContent: string;
    designPath: string;
    designContent: string;
    arrowPath: string;
    arrowContent: string;
}

/**
 * Return the content (and relative paths) for the three files that are
 * scaffolded when a new segment is added.  No I/O — purely computes strings.
 */
export function buildScaffoldContents(
    segmentId: string,
    detail: string,
    specPrefix: string,
): ScaffoldContents {
    const title = segmentId.replace(/-/g, ' ');
    return {
        specsPath: `docs/intent/${segmentId}/${segmentId}-specs.md`,
        specsContent: `---\nprefix: ${specPrefix}\n---\n\n# ${segmentId} specs\n`,
        designPath: `docs/intent/${segmentId}/${segmentId}-design.md`,
        designContent:
            `# ${segmentId} design\n\n` +
            `## Overview\n\n` +
            `<!-- Describe the design for ${segmentId} here. -->\n\n` +
            `## Decisions\n`,
        arrowPath: `docs/arrows/${detail}`,
        arrowContent:
            `# ${title}\n\n## Overview\n\n<!-- Describe the ${segmentId} segment here. -->\n\n` +
            `## References\n\n### LLD\n- \`docs/intent/${segmentId}/${segmentId}-design.md\`\n\n` +
            `### EARS\n- \`docs/intent/${segmentId}/${segmentId}-specs.md\`\n`,
    };
}
