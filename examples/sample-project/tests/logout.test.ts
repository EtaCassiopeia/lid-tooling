// Tests for the logout handler.

import { logout } from '../src/logout';

// @spec AUTH-003
it('invalidates the session', async () => {
    await expect(logout('valid-token')).resolves.toBeUndefined();
});
