# Roadmap

Status of Keytainer's development, and what's likely next. This is a
living document — it records where the project actually is, not a
contract. For the precise per-release record see [CHANGELOG.md](CHANGELOG.md).

Current release: **v0.2.0** (latest).

## Shipped

The original Phase 1–5 plan is complete, plus a v0.2.0 security-hardening
pass beyond it.

### Phase 1 — Vault core ✅ (v0.1.0)
- Master-password vault: Argon2id (m=64 MiB, t=3, p=1) key derivation.
- Single-file encrypted storage with atomic-rename writes (crash-safe).

### Phase 2 — UI MVP ✅ (v0.1.0)
- Setup / Unlock / List / ItemDetail screens.

### Phase 3 — Live secrets & hygiene ✅ (v0.1.0)
- TOTP code generation (RFC 6238, SHA-1/256/512) with live countdown.
- One-click copy with clipboard auto-clear (generation-counter based).
- Auto-lock on idle, emitting a `vault-locked` event to the UI.

### Phase 4 — Power features ✅ (v0.1.0)
- Settings screen.
- Encrypted JSON backup/restore (non-destructive merge import).
- Search + tag-chip filter.
- Optional OS keychain quick-unlock (Secret Service / Keychain /
  Credential Manager).
- Password generator (length + symbols).

### Phase 5 — Packaging & release ✅ (v0.1.0 → v0.1.2)
- Cross-platform installers via GitHub Actions: Linux `.deb`/`.rpm`/
  `.AppImage`, macOS `.dmg` (Apple Silicon + Intel), Windows
  `-setup.exe`/`.msi`.
- Tauri updater signing keys wired into CI; each bundle ships a detached
  `.sig` and the release publishes a `latest.json`.

### v0.2.0 — Security hardening & polish ✅
- **Vault format v2:** XChaCha20-Poly1305 (192-bit nonce) replaces
  AES-256-GCM, removing the practical fixed-key nonce-reuse bound. The
  file header (format version, Argon2 params, salt, nonce) is bound as
  AEAD associated data. Same hardening for the backup envelope (v2).
- **Transparent migration:** v1 vaults and `-v1` backups still open and
  upgrade to v2 on next save — no data loss, no user action.
- **Change master password** in-app (re-derive, re-encrypt, refresh
  keychain).
- **Working in-app updater:** verifies the signed `latest.json` against
  the embedded key before installing.
- **English / 繁體中文** language switch (follows OS locale, persisted).
- Strict CSP; explicit keychain-quick-unlock warning; idle auto-lock no
  longer fires mid-edit; TOTP short-HMAC guard; generated-password buffer
  scrubbed.

## Next up (candidate work, unscheduled)

Gaps versus a mainstream password manager, roughly in priority order.
None of these are committed to a release yet.

### Quality & confidence
- **Frontend tests.** ✅ A Vitest + Testing Library harness now covers the
  i18n resolver, TOTP polling (`TotpDisplay`), lock navigation (`App`), and
  every route — `Setup`, `Unlock`, `List`, `ItemDetail`, and `Settings`
  (error mapping, change-password, locale switch, keychain toggle, updater,
  and backup/restore) — 62 tests in 8 files (`pnpm test`).
- **Backend test gaps.** `session` (idle watcher / lock), `keychain`,
  and `clipboard` auto-clear have no direct tests.

### Features
- **Password strength meter** at setup and on the item form (currently
  only an 8-char minimum).
- **Duplicate / reused password detection** and a basic audit view.
- **Password history** per item.
- **Generator upgrades:** passphrase mode, ambiguous-character exclusion,
  configurable length in the UI (fixed at 20 today).
- **Browser autofill / extension** — the defining gap versus mainstream
  managers (large effort; out of scope for the near term).

### Distribution
- **OS code signing** for macOS (Apple Developer ID) and Windows
  (Authenticode) to remove the first-launch Gatekeeper / SmartScreen
  warnings. Requires paid certificates; the updater signing already in
  place is a *different* mechanism (it authenticates update payloads, not
  the installer at OS level).
- **Release workflow fix:** the `tauri-action` matrix currently races and
  can produce two draft releases per tag with split `latest.json`
  (see the note in [README](README.md#cutting-a-release)). Make the
  workflow publish a single release with a merged `latest.json`.

## Non-goals

By design, Keytainer does **not** plan to add:
- Cloud sync or hosted accounts (local-only is the point).
- Mobile builds.
- Protection against malware already running as the local user (see the
  threat model in the [README](README.md#threat-model)).
