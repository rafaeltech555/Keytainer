# Keytainer

A local-only password manager. Stores site name, account, password, and 2FA secrets in a single encrypted file on your machine. No cloud, no account.

Built with [Tauri 2](https://tauri.app) (Rust backend + React/TypeScript frontend).

## Status

v0.1.0 — first packaged release.

## Features

- **Master password** — Argon2id KDF + AES-256-GCM authenticated encryption
- **TOTP code generation** (RFC 6238, SHA-1/256/512) with countdown ring
- **One-click copy** for passwords and TOTP codes; clipboard auto-clears after 30 s (configurable)
- **Auto-lock** after 5 min of inactivity (configurable)
- **Search + tag filter** across the vault
- **Encrypted JSON backup** — export and import with a separate password
- **OS keychain fast-unlock** (optional) — Linux Secret Service / macOS Keychain / Windows Credential Manager
- **Built-in password generator** (length + symbols toggle)

## Install

Pre-built installers are on the [releases page](https://github.com/rafaeltech555/Keytainer/releases).

### Linux

Either:
```bash
sudo dpkg -i keytainer_0.1.0_amd64.deb
```
or download the `.AppImage`, `chmod +x` it, and run.

### macOS

Download the `.dmg`, open it, drag Keytainer to Applications. The first launch will be blocked by Gatekeeper — **right-click the app → Open → Open anyway**. The binary is **unsigned** because the project doesn't have an Apple Developer cert; you only need the bypass once.

### Windows

Download the `-setup.exe`. Windows SmartScreen will warn "Windows protected your PC" — click **More info → Run anyway**. The binary is **unsigned** for the same reason. You only need the bypass once.

## Threat model

Protects your vault against an attacker who can read `vault.dat` from disk (lost laptop, leaked backup) **as long as your master password is strong**. Does **not** protect against malware already running as your user account.

## Development

Requires:
- Rust stable (1.95+)
- Node 20 + pnpm
- Linux system libs (Ubuntu 22.04/24.04):
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

Build a release locally:
```bash
pnpm tauri build
# → src-tauri/target/release/bundle/{deb,appimage,...}/
```

Run the Rust test suite:
```bash
cd src-tauri && cargo test
```

## Cutting a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

The `.github/workflows/release.yml` GitHub Actions workflow then builds for macOS (aarch64 + x86_64), Linux (x86_64), and Windows (x86_64) on native runners, attaches the installers to a draft release. Edit the draft on GitHub, write notes, and publish.

You can also trigger a dry-run build via the Actions tab → **release → Run workflow**.

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
    totp.rs         RFC 6238 TOTP (SHA-1/256/512)
    backup.rs       Portable JSON backup envelope
    clipboard.rs    Auto-clearing clipboard
    keychain.rs     OS Secret Service / Keychain / Credential Manager
    session.rs      Unlocked session + idle auto-lock
    settings.rs     ~/.config/keytainer/config.json
    commands.rs     #[tauri::command] handlers
    error.rs        AppError → tagged frontend error
```

## Vault file format

```
"KTNR" | version u16 | argon2 params (12 B) | salt[16] | nonce[12] | ct_len u32 | ciphertext
```

Single file at `~/.config/keytainer/vault.dat` (Linux), the platform's equivalent app-data dir elsewhere. Atomic-rename writes — a crash mid-save never destroys the previous good file.

## License

MIT
