// Tests for the login handler.

import { login, UnauthorizedError } from '../src/login';

// @spec AUTH-001
it('creates a session for valid credentials', async () => {
    const token = await login('alice', 'correct-horse-battery-staple');
    expect(token).toBeTruthy();
});

// @spec AUTH-002
it('rejects invalid credentials with UnauthorizedError', async () => {
    await expect(login('alice', 'wrong-password')).rejects.toBeInstanceOf(
        UnauthorizedError,
    );
});
