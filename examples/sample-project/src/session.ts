// Session-store stubs. The session arrow segment is UNMAPPED so
// the real implementation lands later; auth depends on this
// surface only.

export type SessionToken = string;

export async function createSession(_userId: string): Promise<SessionToken> {
    return 'stub-token';
}

export async function invalidateSession(_token: SessionToken): Promise<void> {
    // no-op stub
}

export async function isValid(_token: SessionToken): Promise<boolean> {
    return false;
}
