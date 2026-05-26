// Auth logout handler.

import type { SessionToken } from './session';
import { invalidateSession } from './session';

// @spec AUTH-003
export async function logout(token: SessionToken): Promise<void> {
    await invalidateSession(token);
}
