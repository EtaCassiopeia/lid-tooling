import {
    buildScaffoldContents,
    buildSegmentBlock,
    insertIntoArrows,
    patchSegmentField,
} from './index-yaml';

// Realistic fixture that mirrors the urlshort index.yaml:
// dates as bare YAML strings, comments, inline arrays, taxonomy section.
const REALISTIC_INDEX = `\
schema_version: 2
last_updated: 2026-05-29

taxonomy:
  infrastructure: [storage, auth]
  domain: [shortener-core]

arrows:
  # ── Storage layer ────────────────────────────────────────────────────────────
  storage:
    status: MAPPED
    sampled: 2026-04-10
    audited: 2026-04-15
    detail: storage.md
    blocks: [shortener-core]
    children: [in-memory-store]
    next: "provision Redis cluster; promote redis-store to MAPPED after infrastructure is ready"

  in-memory-store:
    status: OK
    parent: storage
    audited: 2026-04-22
    detail: storage/in-memory-store.md

  # ── Domain layer ─────────────────────────────────────────────────────────────
  shortener-core:
    status: MAPPED
    sampled: 2026-04-20
    detail: shortener-core.md

  auth:
    status: UNMAPPED
    detail: auth.md
    blocks: [shortener-core]
    next: "implement HMAC-SHA256 API-key middleware"

unmapped:
  docs:
    intent: []
`;

// ── buildSegmentBlock ─────────────────────────────────────────────────────────

describe('buildSegmentBlock', () => {
    it('produces a two-space-indented header', () => {
        const result = buildSegmentBlock('billing', { status: 'UNMAPPED', detail: 'billing/core.md' });
        expect(result.startsWith('  billing:')).toBe(true);
    });

    it('emits fields in declared order: status before detail', () => {
        const result = buildSegmentBlock('billing', { status: 'UNMAPPED', detail: 'billing/core.md' });
        const statusLine = result.indexOf('    status:');
        const detailLine = result.indexOf('    detail:');
        expect(statusLine).toBeLessThan(detailLine);
    });

    it('keeps arrays inline, not as block sequences', () => {
        const result = buildSegmentBlock('billing', {
            status: 'UNMAPPED',
            detail: 'billing/core.md',
            blocks: ['api', 'auth'],
        });
        expect(result).toContain('    blocks: [api, auth]');
        expect(result).not.toContain('    - api');
    });

    it('omits empty arrays', () => {
        const result = buildSegmentBlock('billing', {
            status: 'UNMAPPED',
            detail: 'billing/core.md',
            blocks: [],
            children: [],
        });
        expect(result).not.toContain('blocks');
        expect(result).not.toContain('children');
    });

    it('quotes values containing YAML-special characters', () => {
        const result = buildSegmentBlock('billing', {
            status: 'UNMAPPED',
            detail: 'billing/core.md',
            next: 'implement auth: see issue #42',
        });
        expect(result).toContain('"implement auth: see issue #42"');
    });

    it('does not quote plain strings', () => {
        const result = buildSegmentBlock('billing', { status: 'UNMAPPED', detail: 'billing/core.md' });
        expect(result).toContain('    status: UNMAPPED');
        expect(result).toContain('    detail: billing/core.md');
    });
});

// ── insertIntoArrows ──────────────────────────────────────────────────────────

describe('insertIntoArrows', () => {
    const block = buildSegmentBlock('billing', { status: 'UNMAPPED', detail: 'billing/core.md' });

    it('inserts before the next top-level key (unmapped:)', () => {
        const result = insertIntoArrows(REALISTIC_INDEX, block);
        const billingIdx = result.indexOf('  billing:');
        const unmappedIdx = result.indexOf('unmapped:');
        expect(billingIdx).toBeGreaterThan(-1);
        expect(billingIdx).toBeLessThan(unmappedIdx);
    });

    it('the new segment appears exactly once', () => {
        const result = insertIntoArrows(REALISTIC_INDEX, block);
        const count = (result.match(/  billing:/g) ?? []).length;
        expect(count).toBe(1);
    });

    it('preserves comments verbatim', () => {
        const result = insertIntoArrows(REALISTIC_INDEX, block);
        expect(result).toContain('# ── Storage layer ────────────────────────────────────────────────────────────');
        expect(result).toContain('# ── Domain layer ─────────────────────────────────────────────────────────────');
    });

    it('does not coerce date strings', () => {
        const result = insertIntoArrows(REALISTIC_INDEX, block);
        // Dates must remain as plain bare YAML strings, not ISO timestamps.
        expect(result).toContain('sampled: 2026-04-10');
        expect(result).toContain('audited: 2026-04-15');
        expect(result).not.toContain('T00:00:00');
    });

    it('preserves inline arrays unchanged', () => {
        const result = insertIntoArrows(REALISTIC_INDEX, block);
        expect(result).toContain('    blocks: [shortener-core]');
        expect(result).toContain('    children: [in-memory-store]');
        expect(result).toContain('  infrastructure: [storage, auth]');
    });

    it('leaves every original line intact', () => {
        const result = insertIntoArrows(REALISTIC_INDEX, block);
        for (const line of REALISTIC_INDEX.split('\n')) {
            expect(result).toContain(line);
        }
    });

    it('appends when arrows: is the last section (no following top-level key)', () => {
        const minimal = 'schema_version: 2\narrows:\n  auth:\n    status: UNMAPPED\n    detail: auth.md\n';
        const result = insertIntoArrows(minimal, block);
        // The block is inserted between the last segment line (empty trailing) and EOF.
        expect(result).toContain('  billing:');
        expect(result.indexOf('  billing:')).toBeGreaterThan(result.indexOf('  auth:'));
    });

    it('does not add blockedBy to existing entries', () => {
        const result = insertIntoArrows(REALISTIC_INDEX, block);
        expect(result).not.toContain('blockedBy');
    });
});

// ── patchSegmentField ─────────────────────────────────────────────────────────

describe('patchSegmentField', () => {
    it('adds a new field to the target segment', () => {
        const result = patchSegmentField(REALISTIC_INDEX, 'auth', 'parent', 'billing');
        expect(result).toContain('    parent: billing');
    });

    it('replaces an existing field value', () => {
        const result = patchSegmentField(REALISTIC_INDEX, 'auth', 'status', 'MAPPED');
        const authBlock = result.slice(result.indexOf('  auth:'));
        expect(authBlock.split('\n')[1]).toBe('    status: MAPPED');
        // Ensure original status is replaced, not duplicated.
        expect((result.match(/    status: /g) ?? []).length).toBe(
            (REALISTIC_INDEX.match(/    status: /g) ?? []).length,
        );
    });

    it('does not touch other segments', () => {
        const result = patchSegmentField(REALISTIC_INDEX, 'auth', 'parent', 'billing');
        const storageBlock = result.slice(
            result.indexOf('  storage:'),
            result.indexOf('  in-memory-store:'),
        );
        expect(storageBlock).not.toContain('parent: billing');
    });

    it('returns content unchanged when segment is not found', () => {
        const result = patchSegmentField(REALISTIC_INDEX, 'nonexistent', 'parent', 'billing');
        expect(result).toBe(REALISTIC_INDEX);
    });

    it('preserves all other lines exactly', () => {
        const result = patchSegmentField(REALISTIC_INDEX, 'auth', 'parent', 'billing');
        for (const line of REALISTIC_INDEX.split('\n')) {
            if (line !== '  auth:' && !line.startsWith('    ')) {
                expect(result).toContain(line);
            }
        }
    });
});

// ── buildScaffoldContents ─────────────────────────────────────────────────────

describe('buildScaffoldContents', () => {
    const contents = buildScaffoldContents('billing', 'billing/core.md', 'USH-BILL');

    it('specs path follows docs/intent/{id}/{id}-specs.md convention', () => {
        expect(contents.specsPath).toBe('docs/intent/billing/billing-specs.md');
    });

    it('design path follows docs/intent/{id}/{id}-design.md convention', () => {
        expect(contents.designPath).toBe('docs/intent/billing/billing-design.md');
    });

    it('arrow path mirrors the detail argument', () => {
        expect(contents.arrowPath).toBe('docs/arrows/billing/core.md');
    });

    it('specs content includes YAML frontmatter with the prefix', () => {
        expect(contents.specsContent).toContain('---\nprefix: USH-BILL\n---');
    });

    it('design content includes required ## Overview and ## Decisions sections', () => {
        expect(contents.designContent).toContain('## Overview');
        expect(contents.designContent).toContain('## Decisions');
    });

    it('arrow doc includes ## References section with links to intent files', () => {
        expect(contents.arrowContent).toContain('## References');
        expect(contents.arrowContent).toContain('docs/intent/billing/billing-design.md');
        expect(contents.arrowContent).toContain('docs/intent/billing/billing-specs.md');
    });

    it('replaces hyphens with spaces in the arrow doc title', () => {
        const { arrowContent } = buildScaffoldContents('billing-service', 'billing/core.md', 'USH-BILL');
        expect(arrowContent.startsWith('# billing service\n')).toBe(true);
    });
});
