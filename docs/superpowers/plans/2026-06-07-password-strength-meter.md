# Password Strength Meter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a live password-strength meter at the three password-entry points (master-password setup, master-password change, per-item form), with a hard score≥2 gate on the master password and a soft confirm-to-save gate on item passwords.

**Architecture:** Frontend-only. A `src/lib/strength.ts` module configures `@zxcvbn-ts` once and exposes a synchronous `scorePassword()`. A reusable `<StrengthMeter>` component renders a segmented bar + localized label. Setup/Settings gate submission on the score; ItemDetail shows a soft confirm-to-save warning.

**Tech Stack:** React + TypeScript, Vitest + Testing Library, `@zxcvbn-ts/core` + `@zxcvbn-ts/language-common` + `@zxcvbn-ts/language-en`.

**Spec:** `docs/superpowers/specs/2026-06-07-password-strength-meter-design.md`

---

## File structure

- Create `src/lib/strength.ts` — zxcvbn setup + `scorePassword()` + `MIN_MASTER_SCORE`.
- Create `src/lib/strength.test.ts` — strength scoring tests (real zxcvbn).
- Create `src/components/StrengthMeter.tsx` — the meter component.
- Create `src/components/StrengthMeter.test.tsx` — component tests (mocked score).
- Modify `src/lib/i18n.tsx` — add `strength_*`, `pw_too_weak`, `detail_pw_weak_warn`, `detail_save_weak` keys to both `en` and `zh-TW`.
- Modify `src/routes/Setup.tsx` + `Setup.test.tsx` — hard score gate.
- Modify `src/routes/Settings.tsx` + `Settings.test.tsx` — hard score gate on change-password.
- Modify `src/routes/ItemDetail.tsx` + `ItemDetail.test.tsx` — soft confirm-to-save gate.
- Modify `src/App.css` — meter styling.
- Modify `CHANGELOG.md`, `ROADMAP.md`, `README.md` — docs.

All commands run from the repo root `/home/finn/sideproject/Keytainer`. Test command: `pnpm test` (one-shot) or `pnpm vitest run <path>` for a single file.

---

## Task 1: Add zxcvbn-ts dependencies

**Files:**
- Modify: `package.json` (via pnpm)

- [ ] **Step 1: Install the packages**

Run:
```bash
pnpm add @zxcvbn-ts/core @zxcvbn-ts/language-common @zxcvbn-ts/language-en
```
Expected: three entries added under `dependencies` in `package.json`, `pnpm-lock.yaml` updated.

- [ ] **Step 2: Verify they resolve**

Run:
```bash
pnpm vitest run --reporter=dot 2>&1 | tail -3
```
Expected: the existing 62 tests still pass (no import errors).

- [ ] **Step 3: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "build: add zxcvbn-ts for password strength scoring"
```

---

## Task 2: Strength module

**Files:**
- Create: `src/lib/strength.ts`
- Test: `src/lib/strength.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/lib/strength.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { scorePassword, MIN_MASTER_SCORE } from "./strength";

describe("scorePassword", () => {
  it("scores a common dictionary password below the master minimum", () => {
    expect(scorePassword("password")).toBeLessThan(MIN_MASTER_SCORE);
  });

  it("scores a long random passphrase at good or above", () => {
    expect(scorePassword("correct-horse-battery-staple-9173")).toBeGreaterThanOrEqual(3);
  });

  it("treats an empty string as the weakest score", () => {
    expect(scorePassword("")).toBe(0);
  });

  it("uses fair (2) as the master-password minimum", () => {
    expect(MIN_MASTER_SCORE).toBe(2);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/lib/strength.test.ts`
Expected: FAIL — cannot resolve `./strength`.

- [ ] **Step 3: Write the module**

Create `src/lib/strength.ts`:
```ts
import { zxcvbn, zxcvbnOptions } from "@zxcvbn-ts/core";
import * as common from "@zxcvbn-ts/language-common";
import * as en from "@zxcvbn-ts/language-en";

// Configure zxcvbn once at module load. The dictionaries and adjacency graphs
// let it recognise dictionary words, l33t-speak, and keyboard walks. We do NOT
// wire up `translations`, because we render our own localized labels off the
// numeric score rather than zxcvbn's English feedback strings.
zxcvbnOptions.setOptions({
  dictionary: { ...common.dictionary, ...en.dictionary },
  graphs: common.adjacencyGraphs,
});

/** zxcvbn strength score: 0 (weakest) … 4 (strongest). */
export type Score = 0 | 1 | 2 | 3 | 4;

/** Minimum score required for a *master* password (Setup / Settings). */
export const MIN_MASTER_SCORE: Score = 2;

/** Estimate password strength. Synchronous; safe to call on each keystroke. */
export function scorePassword(password: string): Score {
  return zxcvbn(password).score as Score;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/lib/strength.test.ts`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/strength.ts src/lib/strength.test.ts
git commit -m "feat: add zxcvbn-based password strength scorer"
```

---

## Task 3: i18n keys

**Files:**
- Modify: `src/lib/i18n.tsx` (the `en` object near line 14, and the `zh-TW` object)

- [ ] **Step 1: Add the keys to the `en` dictionary**

In `src/lib/i18n.tsx`, inside the `const en = { ... }` object, add a new group after the Setup keys (after `setup_create_btn`):
```ts
  // Password strength
  strength_prefix: "Strength:",
  strength_label_0: "Very weak",
  strength_label_1: "Weak",
  strength_label_2: "Fair",
  strength_label_3: "Good",
  strength_label_4: "Strong",
  pw_too_weak: "Password is too weak — add length or variety.",
  detail_pw_weak_warn: "This password is weak. Click Save again to keep it anyway.",
  detail_save_weak: "Save anyway",
```

- [ ] **Step 2: Add the matching keys to the `zh-TW` dictionary**

In the same file, inside the `zh-TW` object, add the parallel group:
```ts
  // Password strength
  strength_prefix: "強度：",
  strength_label_0: "非常弱",
  strength_label_1: "弱",
  strength_label_2: "普通",
  strength_label_3: "良好",
  strength_label_4: "強",
  pw_too_weak: "密碼太弱 — 增加長度或多樣性。",
  detail_pw_weak_warn: "此密碼偏弱，再按一次「儲存」以保留。",
  detail_save_weak: "仍要儲存",
```

- [ ] **Step 3: Verify parity typechecks and tests pass**

Run: `pnpm vitest run src/lib/i18n.test.tsx`
Expected: PASS. (The `Dict` type forces every `en` key to exist in `zh-TW`; a missing key fails the typecheck / test run.)

- [ ] **Step 4: Commit**

```bash
git add src/lib/i18n.tsx
git commit -m "i18n: add password-strength labels (EN + zh-TW)"
```

---

## Task 4: StrengthMeter component

**Files:**
- Create: `src/components/StrengthMeter.tsx`
- Test: `src/components/StrengthMeter.test.tsx`
- Modify: `src/App.css`

- [ ] **Step 1: Write the failing test**

Create `src/components/StrengthMeter.test.tsx`:
```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithI18n } from "../test/render";

const ipc = vi.hoisted(() => ({
  getSystemLocale: vi.fn(),
  getSettings: vi.fn(),
}));
vi.mock("../lib/ipc", () => ({ ipc }));

const strength = vi.hoisted(() => ({
  scorePassword: vi.fn(),
  MIN_MASTER_SCORE: 2,
}));
vi.mock("../lib/strength", () => strength);

import { StrengthMeter } from "./StrengthMeter";

beforeEach(() => {
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue({ locale: "en" });
  strength.scorePassword.mockReturnValue(2);
});

describe("StrengthMeter", () => {
  it("renders nothing for an empty password", () => {
    const { container } = renderWithI18n(<StrengthMeter password="" />);
    expect(container.querySelector(".strength-meter")).toBeNull();
  });

  it("shows the localized label for the computed score", async () => {
    strength.scorePassword.mockReturnValue(2);
    renderWithI18n(<StrengthMeter password="whatever" />);
    expect(await screen.findByText(/Fair/)).toBeInTheDocument();
  });

  it("reflects the score level in the container class", () => {
    strength.scorePassword.mockReturnValue(4);
    const { container } = renderWithI18n(<StrengthMeter password="whatever" />);
    expect(container.querySelector(".strength-meter")?.className).toContain("level-4");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/components/StrengthMeter.test.tsx`
Expected: FAIL — cannot resolve `./StrengthMeter`.

- [ ] **Step 3: Write the component**

Create `src/components/StrengthMeter.tsx`:
```tsx
import { useMemo } from "react";
import { scorePassword } from "../lib/strength";
import { useT, type TKey } from "../lib/i18n";

const SEGMENTS = 4;

interface Props {
  password: string;
}

/** Live password-strength bar + label. Renders nothing for an empty password. */
export function StrengthMeter({ password }: Props) {
  const t = useT();
  const score = useMemo(() => scorePassword(password), [password]);
  if (!password) return null;

  // zxcvbn scores 0..4; always light at least one segment so a score of 0
  // still reads as "something was measured".
  const filled = Math.max(1, score);

  return (
    <div className={`strength-meter level-${score}`} data-testid="strength-meter">
      <div className="strength-bar" aria-hidden="true">
        {Array.from({ length: SEGMENTS }, (_, i) => (
          <span key={i} className={`strength-seg${i < filled ? " filled" : ""}`} />
        ))}
      </div>
      <span className="strength-label">
        {t("strength_prefix")} {t(`strength_label_${score}` as TKey)}
      </span>
    </div>
  );
}
```

- [ ] **Step 4: Add styling**

Append to `src/App.css`:
```css
.strength-meter {
  margin-top: 0.4rem;
}
.strength-bar {
  display: flex;
  gap: 4px;
}
.strength-seg {
  flex: 1;
  height: 5px;
  border-radius: 2px;
  background: #3a3a3a;
}
.strength-meter.level-0 .strength-seg.filled,
.strength-meter.level-1 .strength-seg.filled {
  background: #d9534f;
}
.strength-meter.level-2 .strength-seg.filled {
  background: #e0a800;
}
.strength-meter.level-3 .strength-seg.filled {
  background: #5cb85c;
}
.strength-meter.level-4 .strength-seg.filled {
  background: #2e7d32;
}
.strength-label {
  display: block;
  font-size: 0.8rem;
  margin-top: 0.2rem;
  opacity: 0.85;
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `pnpm vitest run src/components/StrengthMeter.test.tsx`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add src/components/StrengthMeter.tsx src/components/StrengthMeter.test.tsx src/App.css
git commit -m "feat: add StrengthMeter component"
```

---

## Task 5: Setup — hard score gate

**Files:**
- Modify: `src/routes/Setup.tsx`
- Test: `src/routes/Setup.test.tsx`

- [ ] **Step 1: Update the test (mock strength, add the weak-gate case)**

In `src/routes/Setup.test.tsx`, add the strength mock after the existing `vi.mock("../lib/ipc", ...)` block:
```ts
const strength = vi.hoisted(() => ({
  scorePassword: vi.fn(),
  MIN_MASTER_SCORE: 2,
}));
vi.mock("../lib/strength", () => strength);
```
In the `beforeEach`, add a default strong score so the existing tests stay green:
```ts
  strength.scorePassword.mockReturnValue(4);
```
Then add a new test inside `describe("Setup", ...)`:
```ts
  it("keeps create disabled and warns when the password is too weak", async () => {
    strength.scorePassword.mockReturnValue(1);
    const user = userEvent.setup();
    renderWithI18n(<Setup onCreated={vi.fn()} />);
    await user.type(pwField(), "weakish12");
    await user.type(confirmField(), "weakish12");
    expect(
      screen.getByText("Password is too weak — add length or variety."),
    ).toBeInTheDocument();
    expect(createBtn()).toBeDisabled();
  });
```

- [ ] **Step 2: Run test to verify the new case fails**

Run: `pnpm vitest run src/routes/Setup.test.tsx`
Expected: the new test FAILS (the warning text is not rendered yet; button is enabled). Existing tests still PASS.

- [ ] **Step 3: Implement the gate in `Setup.tsx`**

Add imports at the top:
```ts
import { scorePassword, MIN_MASTER_SCORE } from "../lib/strength";
import { StrengthMeter } from "../components/StrengthMeter";
```
Replace the `canSubmit` line (currently line 17-18):
```ts
  const score = scorePassword(pw);
  const tooWeak = pw.length >= 8 && score < MIN_MASTER_SCORE;
  const canSubmit =
    pw.length >= 8 && score >= MIN_MASTER_SCORE && pw === confirm && !busy;
```
In the password `<label>`, after the `<input ... />` (currently lines 46-52), add the meter and hint:
```tsx
          <StrengthMeter password={pw} />
          {tooWeak && (
            <span className="error-inline">{t("pw_too_weak")}</span>
          )}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/routes/Setup.test.tsx`
Expected: PASS (all Setup tests, including the new weak-gate test).

- [ ] **Step 5: Commit**

```bash
git add src/routes/Setup.tsx src/routes/Setup.test.tsx
git commit -m "feat: gate Setup on master-password strength (score >= 2)"
```

---

## Task 6: Settings — hard score gate on change-password

**Files:**
- Modify: `src/routes/Settings.tsx`
- Test: `src/routes/Settings.test.tsx`

- [ ] **Step 1: Update the test (mock strength, add the weak-gate case)**

In `src/routes/Settings.test.tsx`, add after the ipc mock:
```ts
const strength = vi.hoisted(() => ({
  scorePassword: vi.fn(),
  MIN_MASTER_SCORE: 2,
}));
vi.mock("../lib/strength", () => strength);
```
In `beforeEach`, add:
```ts
  strength.scorePassword.mockReturnValue(4);
```
Add a new test inside `describe("Settings — change password", ...)`:
```ts
  it("keeps the change button disabled when the new password is too weak", async () => {
    strength.scorePassword.mockReturnValue(1);
    const user = userEvent.setup();
    renderWithI18n(<Settings {...settingsProps()} />);
    await user.type(screen.getByLabelText("Current password"), "oldpassword");
    await user.type(
      screen.getByLabelText("New password (at least 8 characters)"),
      "weakish12",
    );
    await user.type(screen.getByLabelText("Confirm new password"), "weakish12");
    expect(
      screen.getByText("Password is too weak — add length or variety."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Change password" }),
    ).toBeDisabled();
  });
```
> Note: use the same render/props pattern the other Settings tests use (the existing file already renders `<Settings>` — reuse its exact helper/props rather than inventing `settingsProps()` if one already exists; match the existing tests in the file).

- [ ] **Step 2: Run test to verify the new case fails**

Run: `pnpm vitest run src/routes/Settings.test.tsx`
Expected: the new test FAILS (no warning, button enabled). Existing tests PASS.

- [ ] **Step 3: Implement the gate in `Settings.tsx`**

Add imports at the top:
```ts
import { scorePassword, MIN_MASTER_SCORE } from "../lib/strength";
import { StrengthMeter } from "../components/StrengthMeter";
```
Replace the `newPwOk` line (currently line 216):
```ts
  const newPwScore = scorePassword(newPw);
  const newPwTooWeak = newPw.length >= 8 && newPwScore < MIN_MASTER_SCORE;
  const newPwOk =
    newPw.length >= 8 && newPwScore >= MIN_MASTER_SCORE && newPw === newPw2;
```
Harden the imperative guard in `changePassword()` (currently line 105):
```ts
    if (
      newPw.length < 8 ||
      scorePassword(newPw) < MIN_MASTER_SCORE ||
      newPw !== newPw2 ||
      pwBusy
    )
      return;
```
In the new-password `<label>` (currently lines 289-297), after the `<input ... />`, add:
```tsx
          <StrengthMeter password={newPw} />
          {newPwTooWeak && (
            <span className="error-inline">{t("pw_too_weak")}</span>
          )}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/routes/Settings.test.tsx`
Expected: PASS (all Settings tests).

- [ ] **Step 5: Commit**

```bash
git add src/routes/Settings.tsx src/routes/Settings.test.tsx
git commit -m "feat: gate change-password on master-password strength"
```

---

## Task 7: ItemDetail — soft confirm-to-save gate

**Files:**
- Modify: `src/routes/ItemDetail.tsx`
- Test: `src/routes/ItemDetail.test.tsx`

- [ ] **Step 1: Update the test (mock strength, add the soft-gate case)**

In `src/routes/ItemDetail.test.tsx`, add after the ipc mock:
```ts
const strength = vi.hoisted(() => ({
  scorePassword: vi.fn(),
  MIN_MASTER_SCORE: 2,
}));
vi.mock("../lib/strength", () => strength);
```
In `beforeEach`, add a default strong score (keeps existing save tests green):
```ts
  strength.scorePassword.mockReturnValue(4);
```
Add a new test inside the top-level `describe`:
```ts
  it("warns before saving a weak password and saves on the second click", async () => {
    strength.scorePassword.mockReturnValue(1);
    const user = userEvent.setup();
    renderDetail("new");
    await user.type(screen.getByLabelText("Site name"), "Example");
    await user.type(screen.getByLabelText("Password"), "weakpw");

    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(ipc.addItem).not.toHaveBeenCalled();
    expect(
      screen.getByText(
        "This password is weak. Click Save again to keep it anyway.",
      ),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Save anyway" }));
    expect(ipc.addItem).toHaveBeenCalledTimes(1);
  });
```
> Note: `renderDetail` and the `getByLabelText("Password")` selector follow the existing helpers/labels in this test file — reuse exactly what the file already defines.

- [ ] **Step 2: Run test to verify the new case fails**

Run: `pnpm vitest run src/routes/ItemDetail.test.tsx`
Expected: the new test FAILS (the first Save click already calls `addItem`). Existing tests PASS.

- [ ] **Step 3: Implement the soft gate in `ItemDetail.tsx`**

Add imports at the top:
```ts
import { scorePassword, MIN_MASTER_SCORE } from "../lib/strength";
import { StrengthMeter } from "../components/StrengthMeter";
```
Add state alongside the other `useState` calls (near line 38):
```ts
  const [confirmWeak, setConfirmWeak] = useState(false);
```
In `generate()` (line 89-93), reset the flag after setting the password (generated passwords are strong):
```ts
  async function generate() {
    const pw = await ipc.generatePassword(20, true);
    patch("password", pw);
    setConfirmWeak(false);
    setShowPw(true);
  }
```
In `save()` (line 107), insert the soft gate right after `if (busy) return;` and before `setBusy(true)`:
```ts
    if (busy) return;
    const pw = form.password;
    if (pw && scorePassword(pw) < MIN_MASTER_SCORE && !confirmWeak) {
      setConfirmWeak(true);
      return; // show the warning; a second click confirms
    }
    setBusy(true);
```
In the password `<label>` (lines 192-205), update the password `<input>`'s `onChange` to re-arm the gate, and add the meter + warning after the `.pw-row` div:
```tsx
            <input
              type={showPw ? "text" : "password"}
              value={form.password}
              onChange={(e) => {
                patch("password", e.target.value);
                setConfirmWeak(false);
              }}
            />
```
After the closing `</div>` of `.pw-row` but still inside the password `<label>`:
```tsx
          <StrengthMeter password={form.password} />
          {confirmWeak && (
            <span className="error-inline">{t("detail_pw_weak_warn")}</span>
          )}
```
Update the submit button label (lines 255-257) to show the confirm wording:
```tsx
          <button type="submit" disabled={busy || !form.site_name}>
            {busy
              ? t("detail_saving")
              : confirmWeak
                ? t("detail_save_weak")
                : t("save")}
          </button>
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/routes/ItemDetail.test.tsx`
Expected: PASS (all ItemDetail tests).

- [ ] **Step 5: Commit**

```bash
git add src/routes/ItemDetail.tsx src/routes/ItemDetail.test.tsx
git commit -m "feat: soft confirm-to-save for weak item passwords"
```

---

## Task 8: Docs + full verification

**Files:**
- Modify: `CHANGELOG.md`, `ROADMAP.md`, `README.md`

- [ ] **Step 1: Run the full frontend suite and note the count**

Run: `pnpm test`
Expected: all tests PASS. Record the new total (was 62; this plan adds 4 + 3 + 1 + 1 + 1 = 10 → expect 72). Use the actual number printed.

- [ ] **Step 2: Update CHANGELOG**

In `CHANGELOG.md`, under `## [Unreleased]` → `### Added`, add:
```markdown
- **Password strength meter.** A zxcvbn-based strength bar now appears at
  master-password setup, master-password change, and the per-item password
  field. The master password must reach at least a "Fair" score to be
  accepted; weak item passwords prompt a one-click confirmation before saving.
```

- [ ] **Step 3: Update ROADMAP**

In `ROADMAP.md`, under `### Features`, replace the strength-meter bullet:
```markdown
- **Password strength meter.** ✅ A zxcvbn-ts meter at setup, change-password,
  and the item form; master passwords are gated at score ≥ 2, weak item
  passwords get a soft confirm-to-save.
```

- [ ] **Step 4: Update README test count**

In `README.md`, update the frontend test-count line (currently "62 tests") to the number printed in Step 1.

- [ ] **Step 5: Run the full suite once more**

Run: `pnpm test`
Expected: all PASS, count matches the README.

- [ ] **Step 6: Commit**

```bash
git add CHANGELOG.md ROADMAP.md README.md
git commit -m "docs: record password strength meter; bump test count"
```

---

## Self-review notes

- **Spec coverage:** strength.ts (§2) → Task 2; StrengthMeter (§3) → Task 4; Setup/Settings hard gate (§4) → Tasks 5–6; ItemDetail soft gate (§4) → Task 7; i18n keys (§5) → Task 3; tests (§6) → embedded per task; deps → Task 1; docs → Task 8. All spec sections covered.
- **Existing-test safety:** Setup/Settings/ItemDetail test files use weak literals (`longenough`, `newpassword`, empty item passwords). Mocking `../lib/strength` to score 4 by default in each `beforeEach` keeps the existing assertions valid; new tests override per-case. Existing item-save tests save with an empty password, which never trips the soft gate.
- **Type consistency:** `scorePassword`, `MIN_MASTER_SCORE`, and the `Score` type are defined in Task 2 and used unchanged everywhere; the dynamic i18n key `strength_label_${score}` is cast `as TKey`; all i18n keys used by components are added in Task 3 before first use in Task 4.
</content>
