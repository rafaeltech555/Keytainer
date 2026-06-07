# Password Reuse & Weak-Password Audit — Design

Date: 2026-06-07
Status: Approved (pending implementation)

## Goal

Help users find and fix their weakest links: passwords reused across several
items, and passwords that are simply weak. Surface them in a dedicated audit
screen reachable from the vault list, with each finding linking to the item
so the user can fix it. This implements the "Duplicate / reused password
detection and a basic audit view" item from `ROADMAP.md`.

Scope is **reuse + weak** detection only. Password history is a separate,
later cycle (its own spec → plan → implementation).

## Security boundary (drives the architecture)

`list_items` returns `ItemSummary` values that deliberately **omit the
password** — passwords live in the Rust backend and only leave it via
`get_item` (single item) or `copy_password` (to the clipboard). Loading every
password into the frontend at once to analyse them would put every secret in
the JS heap simultaneously, which this design avoids.

Therefore the audit runs **entirely in the Rust backend**. It compares
passwords internally and returns a report of *which items* have problems —
the report never contains a password value.

## Approach

A pure backend function analyses the unlocked vault and returns a structured
report; a Tauri command exposes it; a dedicated frontend screen renders it.

- **Reuse:** group items by their exact password string (case-sensitive,
  byte-for-byte). Skip empty passwords. Any group with ≥2 items is a reuse
  group.
- **Weak:** score each non-empty password with the Rust `zxcvbn` crate;
  `score < WEAK_SCORE` (= 2) flags the item as weak. This matches the
  frontend strength meter's "fair" cutoff (`MIN_MASTER_SCORE = 2`), so the
  two features agree on what "weak" means.

(Rejected: scoring in the frontend — would require shipping all passwords to
JS, breaking the security boundary. Rejected: a lightweight Rust heuristic —
the user chose zxcvbn for parity with the meter.)

## Backend

### New crate dependency

Add `zxcvbn` (Rust) to `src-tauri/Cargo.toml` as a normal (non-optional)
dependency.

### `src-tauri/src/audit.rs`

A pure function over the vault plus the report types. No I/O, no Tauri — unit
testable directly.

```rust
use serde::Serialize;
use uuid::Uuid;
use crate::vault::Vault;

/// Passwords scoring below this zxcvbn score (0..=4) are flagged weak.
/// Matches the frontend meter's master-password cutoff.
pub const WEAK_SCORE: u8 = 2;

/// A reference to an item in a finding. Never carries the password value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditItemRef {
    pub id: Uuid,
    pub site_name: String,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReuseGroup {
    pub items: Vec<AuditItemRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WeakItem {
    pub item: AuditItemRef,
    pub score: u8, // 0..=1 (anything < WEAK_SCORE)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditReport {
    pub reused: Vec<ReuseGroup>,
    pub weak: Vec<WeakItem>,
}

pub fn audit(vault: &Vault) -> AuditReport { /* see Behavior */ }
```

**Behavior:**
- Iterate `vault.items`, ignoring any item whose `password` is empty.
- **Reuse:** build a map `password -> Vec<AuditItemRef>`; for each entry with
  ≥2 refs, emit a `ReuseGroup`. Group order: by the first item's `site_name`
  (case-insensitive); items within a group keep vault order. (Deterministic
  output for stable tests/UI.)
- **Weak:** for each non-empty-password item, compute the zxcvbn score; if
  `score < WEAK_SCORE`, push a `WeakItem`. Order weak items by `site_name`
  (case-insensitive).
- The shared password value is never stored in the report.

> zxcvbn crate note: call the crate's scoring entry point and read its 0–4
> score. Do not pass site_name/username as user-inputs (parity with the
> frontend meter, which passes none). The exact crate API (return type /
> method name) is pinned during implementation against the resolved version.

### `src-tauri/src/commands.rs`

```rust
#[tauri::command]
pub fn audit_passwords(state: State<'_, AppState>) -> AppResult<audit::AuditReport> {
    state.with_session(|s| Ok(audit::audit(&s.vault)))
}
```

Register the command in the Tauri `invoke_handler` and add `pub mod audit;`
to `lib.rs`. The command is invoked on demand (when the audit screen opens),
not on every list load.

## Frontend

### `src/lib/types.ts`

```ts
export interface AuditItemRef {
  id: string;
  site_name: string;
  username: string;
}
export interface ReuseGroup { items: AuditItemRef[] }
export interface WeakItem { item: AuditItemRef; score: number }
export interface AuditReport { reused: ReuseGroup[]; weak: WeakItem[] }
```

### `src/lib/ipc.ts`

```ts
auditPasswords: () => invoke<AuditReport>("audit_passwords"),
```

### `src/routes/Audit.tsx`

```ts
interface Props {
  onBack: () => void;
  onSelect: (id: string) => void;
}
```

- On mount, calls `ipc.auditPasswords()`; shows a loading state, then the
  report.
- Header: a back button + title (`audit_title`) + a "rescan" button that
  re-fetches.
- Summary line: counts of reuse groups and weak items (`audit_summary`).
- **Reused** section: one card per `ReuseGroup`; a "♻ N items share one
  password" label; each item is a clickable row (site_name + username)
  calling `onSelect(item.id)`.
- **Weak** section: clickable rows, each with a weak pill.
- Empty state (no reused and no weak): `audit_none` ("No problems found ✓").
- Never renders a password value.
- Errors map via the existing `isAppError` pattern.

### Navigation — `src/App.tsx`

- Extend `Screen`: add `{ kind: "audit" }`, and add an origin to detail:
  `{ kind: "detail"; itemId: string | "new"; from?: "list" | "audit" }`.
- `List` gains an `onAudit` prop; its header gets a "Security check" button
  (label only — no count, so the list never triggers an audit) that sets
  `{ kind: "audit" }`.
- `Audit` screen: `onBack` → `{ kind: "list" }`; `onSelect(id)` →
  `{ kind: "detail", itemId: id, from: "audit" }`.
- `ItemDetail`'s `onClose` / `onSaved` / `onDeleted` route back to the
  origin: if `screen.from === "audit"`, return to `{ kind: "audit" }`
  (re-fetches, so fixed items disappear); otherwise `{ kind: "list" }` as
  today. `onSaved`/`onDeleted` still `bumpList()`.

### `src/routes/List.tsx`

Add the "Security check" button to the existing header actions, wired to the
new `onAudit` prop. No other List behavior changes.

### i18n (`src/lib/i18n.tsx`, EN + zh-TW)

| Key | EN | 繁中 |
|-----|----|----|
| `list_audit_btn` | Security check | 安全檢查 |
| `audit_title` | Security check | 安全檢查 |
| `audit_rescan` | Rescan | 重新掃描 |
| `audit_summary` | {reused} reused · {weak} weak | {reused} 組重用 · {weak} 個弱密碼 |
| `audit_reused_section` | Reused passwords | 重用密碼 |
| `audit_weak_section` | Weak passwords | 弱密碼 |
| `audit_group_count` | {count} items share one password | {count} 個項目共用同一組密碼 |
| `audit_weak_pill` | Weak | 弱 |
| `audit_none` | No problems found ✓ | 沒有發現問題 ✓ |
| `audit_loading` | Checking… | 檢查中… |

(`audit_summary` / `audit_group_count` use the existing `{name}`-style
placeholder mechanism in `t`.)

### Styling (`src/App.css`)

Append an `audit-*` block (reuse-group cards, clickable finding rows, the
weak/reuse pills). Reuse existing palette variables; no inline pixel styles
in the component. The chosen look matches
`docs/superpowers/mockups/audit-A-dedicated-screen.html`.

## Testing

**Backend — `src-tauri/src/audit.rs` (`#[cfg(test)]`):**
- Three items sharing one password produce exactly one `ReuseGroup` with
  three refs (correct ids/site_names).
- Two distinct shared passwords produce two separate groups; unique passwords
  produce none.
- A common password (`"password"`) is flagged weak; a strong passphrase is
  not.
- Empty-password items are skipped entirely (neither reused nor weak).
- The report contains no password strings (structural — `AuditReport` has no
  password field; assert via the data, e.g. a reused pair still exposes only
  id/site_name/username).
- Output ordering is deterministic (by site_name).

**Frontend:**
- `src/routes/Audit.test.tsx` — mock `ipc.auditPasswords`:
  - renders a reuse group and a weak list from a fixture report; clicking a
    finding calls `onSelect` with the item id;
  - an empty report renders `audit_none`;
  - never shows a password (fixtures contain none, by design).
- `src/routes/List.test.tsx` (addition) — the "Security check" button calls
  `onAudit`.
- `src/App.test.tsx` (addition, if feasible without overreach) — navigating
  list → audit → detail(from audit) → back returns to audit. (If this proves
  brittle against the existing App test setup, cover the routing via the
  unit-level props instead and note it.)

## Out of scope (YAGNI)

- Inline reuse/weak badges on the list rows (approach B).
- Automatic/background scanning or a list-level issue count.
- "Stale password" (too-long-unchanged) detection.
- One-click password reset/rotation from the audit.
- Showing the numeric weak score in the UI (binary weak/not-weak only).
- Password history (separate later cycle).

## Docs to update on completion

- `CHANGELOG.md` (Unreleased → Added).
- `ROADMAP.md` (mark the reuse/audit bullet done; leave password history as
  next).
- `README.md` if a test-count line needs bumping.
</content>
