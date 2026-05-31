---
title: IntelliJ IDEA Plugin
nav_order: 4
---

# IntelliJ IDEA Plugin

{: .no_toc }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

The plugin auto-activates in any project containing `docs/arrows/index.yaml`. It works in all JetBrains IDEs: IntelliJ IDEA, GoLand, PyCharm, RustRover, WebStorm, and more.

Install from the [JetBrains Marketplace](https://plugins.jetbrains.com/plugin/dev.lid.intellij) — see [Installation]({{ site.baseurl }}/installation#intellij-idea--jetbrains-ides) for all methods.

---

## Language server features

| Feature | Trigger |
|---------|---------|
| **Hover** | Point at `@spec ID` — shows spec text and status |
| **Go to Definition** | `F12` on `@spec ID` — jumps to the spec line in the spec file |
| **Find References** | `Alt+F7` on a spec definition — lists every source citation |
| **Rename** | `Shift+F6` on a spec ID — renames across the spec file and all `@spec` citations |
| **Completion** | Type `@spec ` — autocompletes from all known spec IDs |
| **Diagnostics** | Squiggles on unknown `@spec` references; coverage warnings on spec lines |
| **Syntax highlighting** | `@spec` keyword and spec IDs highlighted in all supported languages; status markers highlighted in `*-specs.md` files |

Supported languages: Java, Kotlin, Groovy, Markdown, YAML, Rust, Python, Go, TypeScript, JavaScript, Scala, C#, Ruby, C++, C, and Shell Script.

---

## Intent Navigator

The **LID Navigator** panel appears automatically on the right side of the IDE for any LID project.

- **Single-click** a segment node → detail panel (status badge, spec progress bar, dependency chips, next/drift prose)
- **Double-click** a segment node → edit mode: change status, cycle spec completion (○ → ✓ → ⊘), edit next/drift notes
- **Right-click** a segment node → context menu: Open Arrow Doc / Open Spec File / Open LLD / Copy ID
- **`+ Segment`** button → add a new segment via overlay form
- **`+ Add spec`** in the panel → append a spec line directly to the spec file
- Toolbar: filter by status, filter by cluster, fuzzy search, fit graph, focus neighbourhood
- Auto-refreshes within 400 ms of any save to `index.yaml` or `docs/intent/` markdown files

---

## Actions

| Action | Location |
|--------|----------|
| **Initialize LID Project** | **Tools → Initialize LID Project** — scaffolds `docs/arrows/index.yaml` and a first segment stub |

---

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `lid.serverPath` | *(bundled)* | Override path to the `lid-lsp` binary |

---

## Plugin signing (for maintainers)

Releasing to the JetBrains Marketplace requires a signed plugin. Generate the key pair once and store the output as GitHub repository secrets.

### Generate the key pair

```sh
# 1. Generate an RSA private key (encrypted)
openssl genpkey -aes-256-cbc -algorithm RSA \
  -out private_encrypted.pem -pkeyopt rsa_keygen_bits:4096

# 2. Strip the passphrase (or keep it and set PRIVATE_KEY_PASSWORD)
openssl rsa -in private_encrypted.pem -out private.pem

# 3. Generate a self-signed certificate (10-year validity)
openssl req -key private.pem -new -x509 -days 3650 -out chain.crt
```

### Set GitHub repository secrets

Go to **Settings → Secrets and variables → Actions** and add:

| Secret | Value |
|--------|-------|
| `JETBRAINS_MARKETPLACE_TOKEN` | Token from [plugins.jetbrains.com](https://plugins.jetbrains.com) → your account → Tokens |
| `CERTIFICATE_CHAIN` | Contents of `chain.crt` |
| `PRIVATE_KEY` | Contents of `private.pem` |
| `PRIVATE_KEY_PASSWORD` | Passphrase (empty string if stripped in step 2) |

Pushing a `v*` tag triggers the release pipeline, which builds a fat-bundle plugin (all four platform `lid-lsp` binaries in one zip), signs it, and publishes it to the Marketplace automatically.
