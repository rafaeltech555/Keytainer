# Password Strength Meter — Design

Date: 2026-06-07
Status: Approved (pending implementation)

## Goal

Give users live feedback on password strength wherever they enter a
password, and stop the *master* password — the key that protects the whole
vault — from being trivially weak. Today the only check is an 8-character
minimum (`Setup.tsx`, `Settings.tsx`); per-item passwords have no check at
all.

This implements the "Password strength meter" item from `ROADMAP.md`
(Features → Quality), scoped to the three password-entry points:
master-password setup, master-password change, and the per-item form.

## Approach

Frontend-only, using the maintained `@zxcvbn-ts` port of Dropbox's zxcvbn.
Strength is a live UI affordance, so it is computed in the browser on each
keystroke — no IPC round-trip and no lag, consistent with the app's other
live affordances (TOTP countdown, confirm-password match). Passwords are
local-only already, so client-side scoring adds no exposure.

(Rejected: a Rust `zxcvbn` crate behind IPC — a round-trip per keystroke is
laggy, the crate's dictionaries lag the JS port, and it breaks the
established "live UI hints live in the frontend" pattern.)

## Components

### `src/lib/strength.ts`

Configures zxcvbn-ts once at module load and exposes a thin, synchronous API.

```ts
import { zxcvbn, zxcvbnOptions } from "@zxcvbn-ts/core";
import * as common from "@zxcvbn-ts/language-common";
import * as en from "@zxcvbn-ts/language-en";

zxcvbnOptions.setOptions({
  dictionary: { ...common.dictionary, ...en.dictionary },
  graphs: common.adjacencyGraphs,
});

export type Score = 0 | 1 | 2 | 3 | 4;

/** Minimum zxcvbn score required for a *master* password (Setup/Settings). */
export const MIN_MASTER_SCORE: Score = 2;

export function scorePassword(password: string): Score {
  return zxcvbn(password).score as Score;
}
```

- Setup runs once (module-level), so it is paid a single time, not per call.
- `zxcvbn()` is synchronous; for typical password lengths it is fast enough
  to run directly on change. No web worker (YAGNI).
- We deliberately do **not** surface zxcvbn's built-in `feedback.warning` /
  `suggestions`, because those strings are English-only and would leak into
  the 繁中 UI. We render our own localized label keyed off the numeric score.

### `src/components/StrengthMeter.tsx`

```ts
interface Props { password: string }
```

- Renders nothing when `password` is empty.
- Computes the score with `useMemo(() => scorePassword(password), [password])`.
- Renders a 4-segment bar (filled segments and color derived from score) and
  a localized label rendered as `{strength_prefix} {strength_label_{0..4}}`
  (e.g. "Strength: Fair" / "強度：普通").
- Importing `scorePassword` from `../lib/strength` lets component tests mock
  it with `vi.mock` to drive any score deterministically.
- Styling lives in `App.css` under a `strength-meter` block with per-level
  modifier classes; no inline pixel values in the component.

## Integration & behavior

| Location | Behavior |
|----------|----------|
| **Setup** (`Setup.tsx`) | `canSubmit = pw.length >= 8 && scorePassword(pw) >= MIN_MASTER_SCORE && pw === confirm && !busy`. `<StrengthMeter password={pw} />` under the password input. When `pw.length >= 8` but score `< MIN_MASTER_SCORE`, show an inline `pw_too_weak` hint. |
| **Settings** (`Settings.tsx`, change master password) | Same gating on `newPw`: `newPw.length >= 8 && scorePassword(newPw) >= MIN_MASTER_SCORE && newPw === newPw2`. Same `<StrengthMeter />` + `pw_too_weak` hint. |
| **ItemDetail** (`ItemDetail.tsx`, per-item password) | `<StrengthMeter password={form.password} />` under the password row (hidden when empty — handled by the component). **Soft** gate: on save, if `form.password` is non-empty and `scorePassword(form.password) < MIN_MASTER_SCORE` and not yet acknowledged, show a `detail_pw_weak_warn` warning, flip the save button to a "confirm" label, and do not submit. A second click submits. The acknowledgement flag resets whenever the password field changes. |

Item passwords are intentionally never hard-blocked: sites impose their own
rules and a user may need to store a weak password they did not choose.

### ItemDetail confirm-to-save detail

- New state: `const [confirmWeak, setConfirmWeak] = useState(false)`.
- In `save()`, before the existing add/update branch:
  ```ts
  const pw = form.password;
  if (pw && scorePassword(pw) < MIN_MASTER_SCORE && !confirmWeak) {
    setConfirmWeak(true);
    return; // show warning, require a second click
  }
  ```
- The password `patch` path resets it: when the password input changes, call
  `setConfirmWeak(false)` so a new value re-arms the gate.
- The submit button shows `t("save")` normally and `t("detail_save_weak")`
  while `confirmWeak` is true; the `detail_pw_weak_warn` text renders inline.

## i18n keys (EN + 繁中)

Add to both dictionaries in `src/lib/i18n.tsx`:

| Key | EN | 繁中 |
|-----|----|----|
| `strength_label_0` | Very weak | 非常弱 |
| `strength_label_1` | Weak | 弱 |
| `strength_label_2` | Fair | 普通 |
| `strength_label_3` | Good | 良好 |
| `strength_label_4` | Strong | 強 |
| `strength_prefix` | Strength: | 強度： |
| `pw_too_weak` | Password is too weak — add length or variety. | 密碼太弱 — 增加長度或多樣性。 |
| `detail_pw_weak_warn` | This password is weak. Click Save again to keep it anyway. | 此密碼偏弱，再按一次「儲存」以保留。 |
| `detail_save_weak` | Save anyway | 仍要儲存 |

## Testing (Vitest)

- `src/lib/strength.test.ts` — real zxcvbn (deterministic): a common password
  (e.g. `"password"`) scores below `MIN_MASTER_SCORE`; a strong passphrase
  scores `>= 3`; assert `MIN_MASTER_SCORE === 2`.
- `src/components/StrengthMeter.test.tsx` — renders nothing for an empty
  password; with `scorePassword` mocked, renders the matching
  `strength_label_*` and the expected filled-segment count/class.
- `Setup.test.tsx` (additions) — with `scorePassword` mocked: a `>= 8`-char
  password scoring `< 2` keeps the create button disabled and shows
  `pw_too_weak`; a password scoring `>= 2` (matching confirm) enables it.
- `Settings.test.tsx` (additions) — same gating for change-master-password.
- `ItemDetail.test.tsx` (additions) — with `scorePassword` mocked to a weak
  score: the first Save click shows `detail_pw_weak_warn` and does **not**
  call `ipc.addItem`; the second click calls it. A strong score saves on the
  first click.

## Out of scope (YAGNI)

- zxcvbn's built-in English feedback/suggestion strings (we use localized
  labels instead).
- Web-worker / async scoring.
- Reused / duplicate password detection and the audit view (separate ROADMAP
  item).
- Any change to the password generator.
- Backend (Rust) strength scoring.

## Docs to update on completion

- `CHANGELOG.md` (Unreleased → Added).
- `ROADMAP.md` (mark the strength-meter bullet done).
- `README.md` only if the frontend test count line needs bumping.
</content>
</invoke>
