# Vendored LID Upstream Fixture

The `docs/` directory under this folder is a snapshot of
<https://github.com/jszmajda/lid>'s `docs/` tree, vendored as an
integration-test fixture for `lidc check`. It exercises the parsers
and checks against a real-world layout that wasn't authored to
satisfy any particular test.

The upstream LID project is licensed MIT-OR-Apache-2.0, matching the
license of this project; see the upstream repository for full author
attribution.

This fixture isn't kept in lockstep with upstream — it's a frozen
snapshot. If upstream evolves, refresh by re-copying `docs/` from a
clean clone.
