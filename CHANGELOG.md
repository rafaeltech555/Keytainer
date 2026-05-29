# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Security
- Master password now flows through the IPC layer as `secrecy::SecretString`
  (zeroed on drop) instead of a plain `String` — the password buffer no
  longer lingers in heap after unlock.
- `Vault`, `VaultItem`, `TotpEntry`, and `ItemInput` derive
  `zeroize::ZeroizeOnDrop`, so decrypted credentials in memory are wiped
  when the session locks or items are freed.
- Intermediate `serde_json` plaintext buffers in `vault::store::save` /
  `load` and `backup::export_to_file` / `import_from_file` are explicitly
  zeroized after use; backup KDF keys are zeroized after encryption.
- Atomic vault writes now `fsync` the parent directory after rename
  (Unix), making the rename durable across power loss on filesystems
  with deferred journal commits.

## [0.1.0] — 2026-05-28

First packaged release.

### Added
- Master-password vault using Argon2id (m=64 MiB, t=3, p=1) + AES-256-GCM
- Single-file encrypted storage at `~/.config/keytainer/vault.dat` with
  atomic-rename writes so a crash never destroys the previous good file
- TOTP code generation per RFC 6238 (SHA-1/256/512), live in the item
  detail view with a countdown ring
- One-click copy for passwords and TOTP codes; clipboard auto-clears after
  30 s (configurable)
- Auto-lock after 5 min of idle (configurable, fires a `vault-locked` event
  to the UI)
- Search box + tag chip filter on the list view
- Encrypted JSON backup (`keytainer-backup-v1`) with its own password —
  export/import via native file dialog; import is a non-destructive merge
- Optional OS keychain fast-unlock (Linux Secret Service / macOS Keychain
  / Windows Credential Manager)
- Password generator (length + symbols toggle)
- Cross-platform installers built via GitHub Actions: Linux `.deb` / `.rpm`
  / `.AppImage`, macOS `.dmg` (Apple Silicon + Intel), Windows `-setup.exe`
  and `.msi`

### Security notes
- macOS and Windows binaries in this release are **unsigned** — Gatekeeper
  and SmartScreen will warn on first launch. See the README for the
  one-time bypass.

[0.1.0]: https://github.com/rafaeltech555/Keytainer/releases/tag/v0.1.0
