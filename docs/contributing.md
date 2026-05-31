---
title: Contributing
nav_order: 8
---

# Contributing

{: .no_toc }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Building from source

### Prerequisites

- Rust stable (via [rustup](https://rustup.rs/))
- Node.js 20+ (for the VS Code extension)
- JDK 21+ (for the IntelliJ plugin)

### Rust workspace

```sh
cargo build --release
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### VS Code extension

```sh
cd extensions/vscode
npm ci && npm run compile
npm test                              # Jest unit tests (YAML helpers, scaffold)
npx @vscode/vsce package --skip-license
```

### IntelliJ plugin

```sh
# Bundle lid-lsp first (CI copies the binary; locally you must do this)
cargo build --release --bin lid-lsp
cp target/release/lid-lsp extensions/intellij/server/lid-lsp

cd extensions/intellij
./gradlew test           # JUnit unit tests (YAML helpers)
./gradlew buildPlugin    # → build/distributions/lid-intellij-<version>.zip
./gradlew runIde         # launch a sandboxed IDE for manual testing
./gradlew verifyPlugin   # IDE compatibility check
```

---

## Pre-push hook

The pre-push hook at `.git-hooks/pre-push` (or `~/.git-hooks/pre-push`) runs `cargo fmt --check` and `cargo test` automatically before every push.

---

## Versioning

lid-tooling versions independently of the upstream LID project.

| lid-tooling | `schema_version` |
|-------------|-----------------|
| 0.2.x | 2 |

`PATCH` increments are tooling-only fixes with no schema changes.

---

## Release pipeline

Pushing a `v*` tag (e.g. `v0.2.4`) triggers the full release workflow:

1. **Build** — Rust binaries compiled for all four platforms (macOS arm64, macOS x64, Linux x64, Windows x64)
2. **Package** — per-platform tarballs/zips with checksums; per-platform VSIX files; per-platform IntelliJ plugin zips
3. **Publish GitHub release** — all assets attached
4. **Publish VS Code extension** — all platform VSIXs published to VS Code Marketplace and Open VSX
5. **Publish IntelliJ plugin** — fat-bundle zip (all four `lid-lsp` binaries) signed and published to JetBrains Marketplace
6. **Update Homebrew tap** — formula updated with new checksums

### Required GitHub secrets

| Secret | Used by |
|--------|---------|
| `VSCE_PAT` | VS Code Marketplace publish |
| `OVSX_PAT` | Open VSX publish (optional) |
| `JETBRAINS_MARKETPLACE_TOKEN` | JetBrains Marketplace publish |
| `CERTIFICATE_CHAIN` | JetBrains plugin signing |
| `PRIVATE_KEY` | JetBrains plugin signing |
| `PRIVATE_KEY_PASSWORD` | JetBrains plugin signing |
| `HOMEBREW_TAP_TOKEN` | Homebrew tap update |

See [IntelliJ plugin signing]({{ site.baseurl }}/intellij#plugin-signing-for-maintainers) for instructions on generating the signing key pair.
