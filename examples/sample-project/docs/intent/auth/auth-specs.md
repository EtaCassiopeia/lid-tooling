# auth specs

**LLD**: docs/intent/auth/auth-design.md

Status markers: `[x]` implemented · `[ ]` active gap · `[D]` deferred

---

## Login

- [x] **AUTH-001**: When the user submits valid credentials, the system SHALL create a session and return a session token.
- [x] **AUTH-002**: When the user submits invalid credentials, the system SHALL respond with status 401 and not create a session.

## Logout

- [x] **AUTH-003**: When the user invokes logout with a valid session token, the system SHALL invalidate the session.

## Lockout

- [ ] **AUTH-004**: When the user fails authentication 5 times within 10 minutes, the system SHALL lock the account for 15 minutes.

## Password reset

- [ ] **AUTH-005**: When the user requests a password reset, the system SHALL email a one-time token valid for 30 minutes.
