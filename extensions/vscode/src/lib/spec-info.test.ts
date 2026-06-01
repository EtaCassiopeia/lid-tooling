import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';

import { buildSpecInfo } from './spec-info';

function mkTmp(): string {
    return fs.mkdtempSync(path.join(os.tmpdir(), 'lid-spec-info-'));
}

function mkdirp(p: string): void {
    fs.mkdirSync(p, { recursive: true });
}

afterEach(() => {
    // Jest handles individual test isolation; tmp dirs are OS-cleaned.
});

describe('buildSpecInfo – decision docs', () => {
    test('returns empty map when intent dir does not exist', () => {
        const result = buildSpecInfo('/nonexistent/docs/intent');
        expect(result.size).toBe(0);
    });

    test('discovers single decision doc for a segment', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth', 'decisions'));
        fs.writeFileSync(path.join(tmp, 'auth', 'decisions', 'token.md'), '# Token Storage\n');

        const result = buildSpecInfo(tmp);
        const auth = result.get('auth');
        expect(auth?.decisionDocs).toHaveLength(1);
        expect(auth?.decisionDocs?.[0]).toContain('token.md');
    });

    test('discovers multiple decision docs sorted alphabetically', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth', 'decisions'));
        fs.writeFileSync(path.join(tmp, 'auth', 'decisions', 'sessions.md'), '# Sessions\n');
        fs.writeFileSync(path.join(tmp, 'auth', 'decisions', 'tokens.md'), '# Tokens\n');

        const result = buildSpecInfo(tmp);
        const docs = result.get('auth')?.decisionDocs ?? [];
        expect(docs).toHaveLength(2);
        expect(path.basename(docs[0]!)).toBe('sessions.md');
        expect(path.basename(docs[1]!)).toBe('tokens.md');
    });

    test('ignores non-.md files inside decisions/', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth', 'decisions'));
        fs.writeFileSync(path.join(tmp, 'auth', 'decisions', 'notes.txt'), 'plain text');
        fs.writeFileSync(path.join(tmp, 'auth', 'decisions', 'arch.md'), '# Arch\n');

        const result = buildSpecInfo(tmp);
        expect(result.get('auth')?.decisionDocs).toHaveLength(1);
    });

    test('decision docs for different segments stay separate', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth', 'decisions'));
        mkdirp(path.join(tmp, 'payment', 'decisions'));
        fs.writeFileSync(path.join(tmp, 'auth', 'decisions', 'token.md'), '# Token\n');
        fs.writeFileSync(path.join(tmp, 'payment', 'decisions', 'stripe.md'), '# Stripe\n');

        const result = buildSpecInfo(tmp);
        expect(result.get('auth')?.decisionDocs).toHaveLength(1);
        expect(result.get('payment')?.decisionDocs).toHaveLength(1);
        expect(result.get('auth')?.decisionDocs?.[0]).not.toContain('stripe');
    });

    test('segment with no decisions/ dir has undefined decisionDocs', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth'));
        fs.writeFileSync(path.join(tmp, 'auth', 'auth-specs.md'), '# auth\n- [x] **AUTH-001**: ok.\n');

        const result = buildSpecInfo(tmp);
        expect(result.get('auth')?.decisionDocs).toBeUndefined();
    });
});

describe('buildSpecInfo – specs', () => {
    test('counts implemented, open, and deferred specs', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth'));
        fs.writeFileSync(
            path.join(tmp, 'auth', 'auth-specs.md'),
            '# auth\n- [x] **AUTH-001**: done.\n- [ ] **AUTH-002**: open.\n- [D] **AUTH-003**: deferred.\n',
        );

        const result = buildSpecInfo(tmp);
        const counts = result.get('auth')?.counts;
        expect(counts?.implemented).toBe(1);
        expect(counts?.open).toBe(1);
        expect(counts?.deferred).toBe(1);
    });

    test('reads spec prefix from YAML frontmatter', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth'));
        fs.writeFileSync(
            path.join(tmp, 'auth', 'auth-specs.md'),
            '---\nprefix: AUTH\n---\n# auth\n- [ ] **AUTH-001**: open.\n',
        );

        const result = buildSpecInfo(tmp);
        expect(result.get('auth')?.specPrefix).toBe('AUTH');
    });

    test('records specFile path', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth'));
        const specPath = path.join(tmp, 'auth', 'auth-specs.md');
        fs.writeFileSync(specPath, '# auth\n');

        const result = buildSpecInfo(tmp);
        expect(result.get('auth')?.specFile).toBe(specPath);
    });

    test('records lldFile path', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth'));
        const lldPath = path.join(tmp, 'auth', 'auth-design.md');
        fs.writeFileSync(lldPath, '# Design\n');

        const result = buildSpecInfo(tmp);
        expect(result.get('auth')?.lldFile).toBe(lldPath);
    });

    test('merges specs and decision docs for the same segment', () => {
        const tmp = mkTmp();
        mkdirp(path.join(tmp, 'auth', 'decisions'));
        fs.writeFileSync(path.join(tmp, 'auth', 'auth-specs.md'), '# auth\n- [ ] **AUTH-001**: open.\n');
        fs.writeFileSync(path.join(tmp, 'auth', 'auth-design.md'), '# Design\n');
        fs.writeFileSync(path.join(tmp, 'auth', 'decisions', 'token.md'), '# Token\n');

        const result = buildSpecInfo(tmp);
        const info = result.get('auth');
        expect(info?.counts.open).toBe(1);
        expect(info?.lldFile).toContain('auth-design.md');
        expect(info?.decisionDocs).toHaveLength(1);
    });
});
