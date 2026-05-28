# Security Policy

## Supported versions

Only the latest released version is supported. Older versions get no
security fixes.

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅        |

## Reporting a vulnerability

If you find a security issue, **please do not open a public GitHub issue**.

Use GitHub's **private security advisory** flow instead:

→ https://github.com/rafaeltech555/Keytainer/security/advisories/new

Include:
- a description of the issue and its impact;
- steps to reproduce, or a proof-of-concept;
- the Keytainer version affected (`Settings → About`, or the installer
  filename);
- your OS and version.

I'll acknowledge receipt within ~7 days and aim to ship a fix within 30
days for high-severity issues. If a fix needs to wait (e.g. it depends on
an upstream Tauri / dependency release), I'll say so and keep you updated.

## What's in scope

- Cryptographic weaknesses in the vault format, KDF, or AEAD usage
- Memory-handling bugs that leak secrets after lock or process exit
- Logic bugs that allow reading items without the correct master password
  or cached key
- Issues that break the integrity of the encrypted backup format

## What's out of scope

- Attacks that require root or malware already running as the local user
- Side-channel attacks against `arboard` / OS clipboard (these are
  inherent — read the threat model in the README)
- Missing code-signing on macOS / Windows binaries (known, documented)
- Denial of service by feeding intentionally malformed vault.dat /
  backup files (these should fail clearly, not crash — but the failure
  mode itself isn't a vulnerability)
