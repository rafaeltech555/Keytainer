# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Changed
- **Linux-only releases for now.** The release workflow publishes only the
  Linux installers (`.deb` / `.rpm` / `.AppImage`). The macOS/Windows matrix
  entries are commented out because they would ship unsigned and warn on
  first launch; they'll be re-enabled once OS code signing is configured.

### Fixed
- **Release workflow race.** The release pipeline is now a three-job pattern
  (create one draft release → matrix builds upload to that release id →
  publish), so a multi-platform build can no longer fork into two draft
  releases with a split `latest.json`. The updater manifest now covers every
  platform in a single release.

### Added
- **Generator upgrade.** The password generator now has an inline panel on
  the item screen with a configurable length, a symbols toggle, an
  avoid-ambiguous-characters toggle, and a passphrase mode (EFF large
  wordlist) with adjustable word count, separator, capitalization, and an
  optional trailing number.
- **Password history.** Each item now keeps its last 10 previous passwords,
  captured automatically when a password changes. View, copy (with clipboard
  auto-clear), or restore a previous password from the item screen. History
  travels inside the encrypted vault and its backups.
- **Password audit.** A "Security check" screen (reachable from the vault
  list) flags reused passwords (two or more items sharing one password) and
  weak passwords (zxcvbn score below "fair"), computed entirely in the Rust
  backend so passwords never leave it. Each finding links to the item to fix.
- **Password strength meter.** A zxcvbn-based strength bar now appears at
  master-password setup, master-password change, and the per-item password
  field. The master password must reach at least a "Fair" score to be
  accepted; weak item passwords prompt a one-click confirmation before saving.
- **Frontend test suite.** Vitest + React Testing Library harness (`pnpm
  test`) — 72 tests covering the i18n resolver, TOTP polling, lock
  navigation, and every route (`Setup`, `Unlock`, `List`, `ItemDetail`,
  `Settings`), including backend-error mapping, change-password, locale
  switch, keychain toggle, the updater, and backup/restore.
- **Backend test coverage** for the previously untested modules: session
  state (lock/unlock, idle-timer bump, and that a failed command does not
  count as activity), clipboard auto-clear generation/staleness logic, and
  keychain key encode/decode with the 32-byte malformed-key guard. The Rust
  suite is now 60 tests.

## [0.2.0] — 2026-05-31

### Security
- **Vault format v2.** Vaults are now encrypted with XChaCha20-Poly1305
  (192-bit random nonce) instead of AES-256-GCM, eliminating the practical
  nonce-reuse birthday bound under a long-lived session key. The file
  header (format version, Argon2 params, salt, nonce) is now bound as
  AEAD associated data, so tampering with KDF params is detected instead
  of being a silent downgrade vector. Same hardening applied to the
  encrypted backup envelope (`keytainer-backup-v2`).
- **Transparent migration.** Existing v1 (AES-GCM) vaults and `…-v1`
  backups still open; a v1 vault is rewritten as v2 on the next save. No
  user action and no data loss.
- Strict Content-Security-Policy replaces the previous `null` CSP.
- The OS-keychain quick-unlock now carries an explicit in-app warning that
  it stores the raw vault key in the OS secret store.
- TOTP dynamic truncation guards against a short HMAC (no panic path).
- The password generator scrubs its working buffer after use.

### Added
- **Change master password** from Settings — re-derives the key with a
  fresh salt, re-encrypts the vault, and refreshes the keychain entry if
  quick-unlock is on.
- **In-app updater.** `tauri-plugin-updater` + `tauri-plugin-process` are
  now wired up; Settings has a "check for updates" action that verifies
  the signed `latest.json`/`.sig` against the embedded public key and can
  download, install, and relaunch.
- **English / 繁體中文 language switch.** UI language follows the OS locale
  by default and can be overridden in Settings; the choice is persisted.

### Fixed
- Idle auto-lock no longer fires mid-edit: raw UI activity (typing,
  pointer) now refreshes the idle timer via a throttled activity ping,
  not only backend IPC calls.

### Changed
- Replaced the hand-rolled base64 codec with the `base64` crate.

## [0.1.2] — 2026-05-31

### Fixed
- The updater public key embedded in the app now matches the private key
  used to sign release artifacts. (The key first wired up for 0.1.1 was
  mismatched, so a future in-app auto-updater could not have verified
  0.1.1 downloads.)
- `package.json` and the Rust crate version now track the release version
  (0.1.1 only bumped `tauri.conf.json`).

## [0.1.1] — 2026-05-31

### Added
- Release artifacts are now signed with a Tauri updater key. Each bundle
  ships a detached `.sig` and the release publishes a `latest.json`, so a
  future in-app auto-updater can verify downloads against the embedded
  public key. (Does not affect OS code signing — macOS/Windows binaries
  remain unsigned for now.)

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

[0.2.0]: https://github.com/rafaeltech555/Keytainer/releases/tag/v0.2.0
[0.1.2]: https://github.com/rafaeltech555/Keytainer/releases/tag/v0.1.2
[0.1.1]: https://github.com/rafaeltech555/Keytainer/releases/tag/v0.1.1
[0.1.0]: https://github.com/rafaeltech555/Keytainer/releases/tag/v0.1.0
