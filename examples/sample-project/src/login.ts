// Auth login handler.

import type { SessionToken } from './session';
import { createSession } from './session';

// @spec AUTH-001, AUTH-002
export async function login(
    username: string,
    password: string,
): Promise<SessionToken> {
    const user = await findUser(username);
    if (user === null || !verifyPassword(user, password)) {
        throw new UnauthorizedError();
    }
    return createSession(user.id);
}

async function findUser(_username: string): Promise<User | null> {
    return null;
}

function verifyPassword(_user: User, _password: string): boolean {
    return false;
}

export class UnauthorizedError extends Error {
    constructor() {
        super('invalid credentials');
    }
}

type User = { id: string };
