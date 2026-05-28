# Keytainer

A local-only password manager. Stores site name, account, password, and 2FA secrets in a single encrypted file on your machine. No cloud, no account.

Built with [Tauri 2](https://tauri.app) (Rust backend + React/TypeScript frontend).

## Status

Pre-alpha. Phase 1 (encrypted vault core) in progress.

## Features (planned)

- Master password — Argon2id KDF + AES-256-GCM authenticated encryption
- Optional OS keychain fast-unlock (Linux Secret Service)
- TOTP code generation (RFC 6238, SHA-1/256/512)
- Search, tags, encrypted JSON backup
- Auto-lock on idle, clipboard auto-clear

## Threat model

Protects your vault against an attacker who can read `vault.dat` from disk (lost laptop, leaked backup) **as long as your master password is strong**. Does **not** protect against malware running as your user account.

## Development

Requires:
- Rust stable (1.95+)
- Node 20 + pnpm
- Linux system libs (Ubuntu 24.04):
  ```
  sudo apt-get install -y libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
    libayatana-appindicator3-dev librsvg2-dev patchelf \
    libdbus-1-dev pkg-config libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
  ```

```bash
pnpm install
pnpm tauri dev
```

Run the Rust test suite:

```bash
cd src-tauri && cargo test
```

## Project layout

```
src/             React + TypeScript frontend
src-tauri/       Rust backend
  src/
    crypto/     Argon2id + AES-256-GCM
    vault/      Vault types, atomic file storage
    totp.rs     RFC 6238 TOTP
    error.rs    AppError
```

## Vault file format

```
"KTNR" | version u16 | argon2 params (12B) | salt[16] | nonce[12] | ct_len u32 | ciphertext
```

Single file at `~/.config/keytainer/vault.dat`. Atomic-rename writes — a crash never destroys the previous good file.

## License

MIT
