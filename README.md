# Keytainer

> A local-only password manager. Stores site name, account, password, and 2FA secrets in a single XChaCha20-Poly1305 encrypted file on your machine. No cloud, no account.

[![Release](https://img.shields.io/github/v/release/rafaeltech555/Keytainer?include_prereleases&sort=semver)](https://github.com/rafaeltech555/Keytainer/releases/latest)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24c8db)](https://tauri.app)

Built with [Tauri 2](https://tauri.app) — Rust backend, React + TypeScript frontend.

---

## Install

Grab the right file for your OS from the **[latest release](https://github.com/rafaeltech555/Keytainer/releases/latest)**:

| OS | File | Notes |
|---|---|---|
| Ubuntu / Debian | `Keytainer_<ver>_amd64.deb` | `sudo dpkg -i ./Keytainer_<ver>_amd64.deb` |
| Fedora / RHEL | `Keytainer-<ver>-1.x86_64.rpm` | `sudo rpm -i ./Keytainer-<ver>-1.x86_64.rpm` |
| Any Linux | `Keytainer_<ver>_amd64.AppImage` | `chmod +x` then double-click |
| macOS Apple Silicon | `Keytainer_<ver>_aarch64.dmg` | one-time Gatekeeper bypass ↓ |
| macOS Intel | `Keytainer_<ver>_x64.dmg` | one-time Gatekeeper bypass ↓ |
| Windows | `Keytainer_<ver>_x64-setup.exe` | one-time SmartScreen bypass ↓ |

### macOS first-launch bypass (one-time)

The binary isn't code-signed (no Apple Developer cert). Gatekeeper will refuse the first launch. To allow it:

1. Open `/Applications`
2. **Right-click** Keytainer → **Open**
3. Click **Open** in the warning dialog

After this, normal double-click works forever.

### Windows first-launch bypass (one-time)

SmartScreen will show "Windows protected your PC". Click **More info → Run anyway**. Same reason as macOS — no code-signing cert.

## Quick start

1. **First launch** → set a master password (≥ 8 chars). Losing it means losing the vault — there is no recovery.
2. Click **＋ 新增** to add an item: fill in site name, account, password (or hit *產生* for a strong random one), optionally a TOTP secret.
3. From the list, open any item to see its **live TOTP code** with a countdown, and to **複製密碼** / **複製 2FA** — the clipboard auto-clears after 30 seconds.
4. Click the **⚙** icon to adjust auto-lock timeout, clipboard clear timeout, enable **OS keychain quick-unlock**, or export / import an encrypted backup.

## Features

- **Master password** — Argon2id KDF (OWASP 2024 params) → XChaCha20-Poly1305 AEAD, with the file header bound as associated data
- **Change the master password** in-app — re-derives the key and re-encrypts the vault
- **TOTP code generation** (RFC 6238, SHA-1 / SHA-256 / SHA-512) with live countdown
- **Clipboard auto-clear** after 30 s (configurable, generation-counter-based so a newer copy supersedes an older scheduled clear)
- **Auto-lock on idle** with `vault-locked` event back to the UI (UI activity keeps it alive, so you're not locked out mid-edit)
- **Search + tag filter** across the vault
- **Encrypted backup** (`keytainer-backup-v2`) — non-destructive merge on import; older `-v1` backups still import
- **Optional OS keychain quick-unlock** (Secret Service / Keychain / Credential Manager)
- **Password generator** (length + symbols)
- **English / 繁體中文** — follows your OS locale, switchable in Settings
- **Signed in-app updates** — checks the signed `latest.json`, verifies against the embedded key, installs and relaunches

See [CHANGELOG.md](CHANGELOG.md) for what landed in each version.

## Threat model

Protects the vault against an attacker who can read `vault.dat` from disk (lost laptop, stolen backup) **as long as your master password is strong**. **Does not** protect against malware already running as your user account.

Decrypted credentials and the master password are wrapped in `zeroize`-on-drop types, so they're wiped from heap as soon as the session locks. This is best-effort hygiene, not a defence — if your machine is swapping or an attacker has live root, those pages can still leak.

Vault file location:
- Linux: `~/.config/keytainer/vault.dat`
- macOS: `~/Library/Application Support/com.rafaeltech555.keytainer/vault.dat`
- Windows: `%APPDATA%\com.rafaeltech555.keytainer\vault.dat`

Security-issue reporting: see [SECURITY.md](SECURITY.md) — use GitHub's private advisory flow, not public issues.

## Vault file format

```
"KTNR" | version u16 | argon2 params (12 B) | salt[16] | nonce[N] | ct_len u32 | ciphertext
```

- **v2** (current): `nonce` is 24 bytes and the payload is encrypted with XChaCha20-Poly1305; the entire header (magic … ct_len) is passed as AEAD associated data, so tampering with the stored Argon2 params is detected rather than silently honoured. **v1** (legacy): 12-byte nonce, AES-256-GCM, no AAD — still readable, and upgraded to v2 on the next save.
- Argon2id params (m_cost_kib, t_cost, p_cost) are persisted in the file so we can re-derive the same key on load even if the global defaults later change
- Writes go via `vault.dat.tmp` → `fsync` → atomic `rename`, so a crash mid-write never destroys the previous good file

## Build from source

Requires:
- Rust stable (1.95+)
- Node 20 + pnpm
- Linux system libs (Ubuntu 22.04 / 24.04):
  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
    libayatana-appindicator3-dev librsvg2-dev patchelf \
    libdbus-1-dev pkg-config libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
  ```

Run in dev mode:
```bash
pnpm install
pnpm tauri dev
```

Build a local release:
```bash
pnpm tauri build
# → src-tauri/target/release/bundle/{deb,rpm,appimage,...}/
```

Run the Rust test suite (33 tests):
```bash
cd src-tauri && cargo test
```

## Cutting a release

```bash
git tag v<x.y.z>
git push origin v<x.y.z>
```

`.github/workflows/release.yml` then builds for macOS (aarch64 + x86_64), Linux (x86_64), and Windows (x86_64) on native runners, and uploads the installers to a **draft** release on GitHub. Edit the draft, write notes (see prior releases for the template), then publish.

Each artifact is signed with the Tauri updater (minisign) key — CI secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, public key embedded in `tauri.conf.json` — so every bundle ships a detached `.sig` and the release publishes a `latest.json`. The in-app updater (Settings → Updates) checks `latest.json` and verifies the signature against the embedded public key before installing. Note: this signs update *payloads*; it is **not** OS code signing, so macOS/Windows still warn on first launch.

A workflow_dispatch trigger is also available for dry-runs that won't fight the production tag.

## Project layout

```
src/                React + TypeScript frontend
  routes/           Setup / Unlock / List / ItemDetail / Settings
  components/       TotpDisplay (with countdown ring)
  lib/              IPC wrapper, TS types

src-tauri/          Rust backend
  src/
    crypto/         Argon2id + AES-256-GCM
    vault/          Vault types, atomic-write encrypted storage
    totp.rs         RFC 6238 TOTP (SHA-1 / 256 / 512)
    backup.rs       Portable JSON backup envelope
    clipboard.rs    Auto-clearing clipboard
    keychain.rs     OS Secret Service / Keychain / Credential Manager
    session.rs      Unlocked session state + idle auto-lock
    settings.rs     ~/.config/keytainer/config.json
    commands.rs     #[tauri::command] handlers
    error.rs        AppError → tagged frontend error
```

## License

[MIT](LICENSE).
