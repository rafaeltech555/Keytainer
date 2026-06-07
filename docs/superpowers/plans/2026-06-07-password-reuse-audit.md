# 密碼重用與弱密碼 Audit 實作計畫

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Rust 後端分析金庫,找出重用與弱密碼,透過一個從清單進入的專屬 audit 畫面呈現;密碼永不離開後端。

**Architecture:** 純後端函式 `audit(&Vault) -> AuditReport` 比對密碼並回傳「哪些項目」有問題(不含密碼值);Tauri 命令 `audit_passwords` 暴露它;前端 `Audit.tsx` 渲染報告,點項目開 ItemDetail 去修,修完回 audit。

**Tech Stack:** Rust + Tauri 2、`zxcvbn` crate(後端評分)、React + TypeScript、Vitest + Testing Library。

**Spec:** `docs/superpowers/specs/2026-06-07-password-reuse-audit-design.md`

---

## 檔案結構

- 新增 `src-tauri/src/audit.rs` —— 純函式 + 報告型別 + `#[cfg(test)]` 測試。
- 改 `src-tauri/src/lib.rs` —— 加 `pub mod audit;`,在 `invoke_handler` 註冊新命令。
- 改 `src-tauri/src/commands.rs` —— 加 `audit_passwords` 命令。
- 改 `src-tauri/Cargo.toml` —— 加 `zxcvbn` 依賴。
- 改 `src/lib/types.ts`、`src/lib/ipc.ts` —— 型別與 binding。
- 改 `src/lib/i18n.tsx` —— `audit_*` 字串(EN + zh-TW)。
- 新增 `src/routes/Audit.tsx` + `src/routes/Audit.test.tsx`。
- 改 `src/App.tsx`(導航)、`src/routes/List.tsx`(按鈕)、`src/App.css`(樣式)。
- 測試:`src/routes/List.test.tsx`、`src/App.test.tsx` 各加一案。
- 文件:`CHANGELOG.md`、`ROADMAP.md`、`README.md`。

指令一律從 repo 根目錄 `/home/finn/sideproject/Keytainer` 執行。後端測試從 `src-tauri/` 跑(`cargo test`);前端用 `pnpm vitest run <path>`。所有 git commit 訊息結尾加上:
```
Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 1：後端 audit 模組(純函式 + 型別 + 測試)

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/audit.rs`
- Modify: `src-tauri/src/lib.rs`（加 `pub mod audit;`)

- [ ] **Step 1: 加入 zxcvbn 依賴**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 區、`# Errors` 之前加一行(自成一個註解小節):
```toml
# Password strength scoring (audit)
zxcvbn = "3"
```

- [ ] **Step 2: 在 lib.rs 宣告模組**

`src-tauri/src/lib.rs` 模組宣告區目前依字母序排列(`pub mod backup;` … `pub mod vault;`)。在 `pub mod backup;` 之後加:
```rust
pub mod audit;
```

- [ ] **Step 3: 寫失敗測試**

建立 `src-tauri/src/audit.rs`,先只放型別宣告與一個 `todo!()` 的函式,連同測試:
```rust
use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use crate::vault::{Vault, VaultItem};

/// Passwords scoring below this zxcvbn score (0..=4) are flagged weak.
/// Matches the frontend meter's master-password cutoff (MIN_MASTER_SCORE).
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
    pub score: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditReport {
    pub reused: Vec<ReuseGroup>,
    pub weak: Vec<WeakItem>,
}

fn item_ref(i: &VaultItem) -> AuditItemRef {
    AuditItemRef {
        id: i.id,
        site_name: i.site_name.clone(),
        username: i.username.clone(),
    }
}

/// zxcvbn score (0..=4) for a password.
fn strength_score(password: &str) -> u8 {
    // zxcvbn v3: `zxcvbn(pw, &[])` returns `Entropy`; `.score()` is a
    // fieldless `Score` enum castable to u8. (If the resolved crate version
    // differs — e.g. 2.x returns `Result<Entropy>` and `.score()` is already
    // u8 — adapt only this one helper.)
    zxcvbn::zxcvbn(password, &[]).score() as u8
}

pub fn audit(_vault: &Vault) -> AuditReport {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;

    fn item(name: &str, user: &str, pw: &str) -> VaultItem {
        VaultItem {
            id: Uuid::new_v4(),
            site_name: name.into(),
            username: user.into(),
            password: pw.into(),
            totp: None,
            url: None,
            notes: None,
            tags: vec![],
            created_at: 0,
            updated_at: 0,
        }
    }

    fn vault(items: Vec<VaultItem>) -> Vault {
        let mut v = Vault::default();
        v.items = items;
        v
    }

    #[test]
    fn groups_items_sharing_one_password() {
        let v = vault(vec![
            item("GitHub", "a", "hunter2sameshared"),
            item("GitLab", "b", "hunter2sameshared"),
            item("Netflix", "c", "hunter2sameshared"),
        ]);
        let report = audit(&v);
        assert_eq!(report.reused.len(), 1);
        assert_eq!(report.reused[0].items.len(), 3);
    }

    #[test]
    fn distinct_shared_passwords_make_separate_groups() {
        let v = vault(vec![
            item("A", "a", "sharedAAAA1111"),
            item("B", "b", "sharedAAAA1111"),
            item("C", "c", "sharedBBBB2222"),
            item("D", "d", "sharedBBBB2222"),
            item("E", "e", "uniqueEEEE3333"),
        ]);
        let report = audit(&v);
        assert_eq!(report.reused.len(), 2);
    }

    #[test]
    fn flags_weak_but_not_strong_passwords() {
        let v = vault(vec![
            item("Weak", "a", "password"),
            item("Strong", "b", "correct-horse-battery-staple-9173"),
        ]);
        let report = audit(&v);
        let weak_sites: Vec<&str> = report.weak.iter().map(|w| w.item.site_name.as_str()).collect();
        assert!(weak_sites.contains(&"Weak"));
        assert!(!weak_sites.contains(&"Strong"));
    }

    #[test]
    fn skips_empty_passwords_entirely() {
        let v = vault(vec![
            item("NoPass1", "a", ""),
            item("NoPass2", "b", ""),
        ]);
        let report = audit(&v);
        assert!(report.reused.is_empty());
        assert!(report.weak.is_empty());
    }

    #[test]
    fn output_is_sorted_by_site_name() {
        let v = vault(vec![
            item("Zebra", "a", "password"),
            item("Apple", "b", "password"),
        ]);
        let report = audit(&v);
        // Both weak AND reused; check the weak ordering is deterministic.
        assert_eq!(report.weak[0].item.site_name, "Apple");
        assert_eq!(report.weak[1].item.site_name, "Zebra");
    }
}
```

- [ ] **Step 4: 跑測試確認失敗**

Run: `cd src-tauri && cargo test --lib audit::`
Expected: 編譯成功但測試 panic（`todo!()`）—— FAIL。若編譯錯誤指向 `zxcvbn` 的 API(`score()` 型別等),調整 `strength_score` 這一個 helper 後再跑,直到只剩 `todo!()` 的 panic。

- [ ] **Step 5: 實作 audit()**

把 `pub fn audit` 換成:
```rust
pub fn audit(vault: &Vault) -> AuditReport {
    // Reuse: group by exact password, skipping empty passwords. Items keep
    // vault order within a group; groups are sorted by the first item's name.
    let mut groups: HashMap<&str, Vec<AuditItemRef>> = HashMap::new();
    for it in &vault.items {
        if it.password.is_empty() {
            continue;
        }
        groups.entry(it.password.as_str()).or_default().push(item_ref(it));
    }
    let mut reused: Vec<ReuseGroup> = groups
        .into_values()
        .filter(|items| items.len() >= 2)
        .map(|items| ReuseGroup { items })
        .collect();
    reused.sort_by(|a, b| {
        a.items[0]
            .site_name
            .to_lowercase()
            .cmp(&b.items[0].site_name.to_lowercase())
    });

    // Weak: score each non-empty password; flag those below WEAK_SCORE.
    let mut weak: Vec<WeakItem> = vault
        .items
        .iter()
        .filter(|i| !i.password.is_empty())
        .filter_map(|i| {
            let score = strength_score(&i.password);
            (score < WEAK_SCORE).then(|| WeakItem {
                item: item_ref(i),
                score,
            })
        })
        .collect();
    weak.sort_by(|a, b| {
        a.item
            .site_name
            .to_lowercase()
            .cmp(&b.item.site_name.to_lowercase())
    });

    AuditReport { reused, weak }
}
```

- [ ] **Step 6: 跑測試確認通過**

Run: `cd src-tauri && cargo test --lib audit::`
Expected: PASS（5 個測試)。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/audit.rs src-tauri/src/lib.rs
git commit -m "feat(audit): backend reuse/weak password analysis"
```

---

## Task 2：後端命令 audit_passwords

**Files:**
- Modify: `src-tauri/src/commands.rs`（檔尾加命令)
- Modify: `src-tauri/src/lib.rs`（在 `invoke_handler` 註冊)

- [ ] **Step 1: 加入命令**

在 `src-tauri/src/commands.rs` 檔尾加:
```rust
#[tauri::command]
pub fn audit_passwords(state: State<'_, AppState>) -> AppResult<crate::audit::AuditReport> {
    state.with_session(|s| Ok(crate::audit::audit(&s.vault)))
}
```
（`State`、`AppState`、`AppResult` 已是該檔案既有的 import。)

- [ ] **Step 2: 註冊命令**

在 `src-tauri/src/lib.rs` 的 `tauri::generate_handler![ ... ]` 清單中,於 `commands::generate_password,` 之後加一行:
```rust
            commands::audit_passwords,
```

- [ ] **Step 3: 編譯 + 既有測試**

Run: `cd src-tauri && cargo test --lib`
Expected: 全部 PASS（含 Task 1 的 5 個新測試;總數 65)。無編譯警告新增。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(audit): expose audit_passwords command"
```

---

## Task 3：前端型別與 ipc binding

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/ipc.ts`

- [ ] **Step 1: 加型別**

在 `src/lib/types.ts` 檔尾加:
```ts
export interface AuditItemRef {
  id: string;
  site_name: string;
  username: string;
}
export interface ReuseGroup {
  items: AuditItemRef[];
}
export interface WeakItem {
  item: AuditItemRef;
  score: number;
}
export interface AuditReport {
  reused: ReuseGroup[];
  weak: WeakItem[];
}
```

- [ ] **Step 2: 加 ipc binding**

在 `src/lib/ipc.ts` 檔案頂部的型別 import 加入 `AuditReport`(沿用該檔現有的 `import type { ... } from "./types";` 形式),並在 `ipc` 物件中、`generatePassword` 之後加:
```ts
  auditPasswords: () => invoke<AuditReport>("audit_passwords"),
```

- [ ] **Step 3: 型別檢查**

Run: `pnpm tsc --noEmit`
Expected: 無錯誤。

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/ipc.ts
git commit -m "feat(audit): frontend types and ipc binding"
```

---

## Task 4：i18n 字串

**Files:**
- Modify: `src/lib/i18n.tsx`（`en` 與 `zh-TW` 兩個字典)

- [ ] **Step 1: 在 `en` 字典加入**

在 `en` 物件中(放在 List 相關鍵附近的合理位置,新增一個註解小節):
```ts
  // Audit
  list_audit_btn: "Security check",
  audit_title: "Security check",
  audit_rescan: "Rescan",
  audit_summary: "{reused} reused · {weak} weak",
  audit_reused_section: "Reused passwords",
  audit_weak_section: "Weak passwords",
  audit_group_count: "{count} items share one password",
  audit_weak_pill: "Weak",
  audit_none: "No problems found ✓",
  audit_loading: "Checking…",
```

- [ ] **Step 2: 在 `zh-TW` 字典加入對應**

```ts
  // Audit
  list_audit_btn: "安全檢查",
  audit_title: "安全檢查",
  audit_rescan: "重新掃描",
  audit_summary: "{reused} 組重用 · {weak} 個弱密碼",
  audit_reused_section: "重用密碼",
  audit_weak_section: "弱密碼",
  audit_group_count: "{count} 個項目共用同一組密碼",
  audit_weak_pill: "弱",
  audit_none: "沒有發現問題 ✓",
  audit_loading: "檢查中…",
```

- [ ] **Step 3: 驗證 parity**

Run: `pnpm vitest run src/lib/i18n.test.tsx`
Expected: PASS（`Dict` 型別強制兩字典鍵一致)。

- [ ] **Step 4: Commit**

```bash
git add src/lib/i18n.tsx
git commit -m "i18n: add audit screen strings (EN + zh-TW)"
```

---

## Task 5：Audit 畫面 + 測試 + 樣式

**Files:**
- Create: `src/routes/Audit.tsx`
- Test: `src/routes/Audit.test.tsx`
- Modify: `src/App.css`

- [ ] **Step 1: 寫失敗測試**

建立 `src/routes/Audit.test.tsx`:
```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithI18n } from "../test/render";
import type { AuditReport } from "../lib/types";

const ipc = vi.hoisted(() => ({
  auditPasswords: vi.fn(),
  getSystemLocale: vi.fn(),
  getSettings: vi.fn(),
}));
vi.mock("../lib/ipc", () => ({ ipc }));

import { Audit } from "./Audit";

beforeEach(() => {
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue({ locale: "en" });
});

const report = (over: Partial<AuditReport> = {}): AuditReport => ({
  reused: [],
  weak: [],
  ...over,
});

describe("Audit", () => {
  it("shows the no-problems message for a clean report", async () => {
    ipc.auditPasswords.mockResolvedValue(report());
    renderWithI18n(<Audit onBack={vi.fn()} onSelect={vi.fn()} />);
    expect(await screen.findByText("No problems found ✓")).toBeInTheDocument();
  });

  it("renders reuse groups and weak items, selecting on click", async () => {
    ipc.auditPasswords.mockResolvedValue(
      report({
        reused: [
          {
            items: [
              { id: "a", site_name: "GitHub", username: "alice" },
              { id: "b", site_name: "GitLab", username: "alice" },
            ],
          },
        ],
        weak: [{ item: { id: "c", site_name: "Forum", username: "nick" }, score: 1 }],
      }),
    );
    const onSelect = vi.fn();
    const user = userEvent.setup();
    renderWithI18n(<Audit onBack={vi.fn()} onSelect={onSelect} />);

    expect(await screen.findByText("Reused passwords")).toBeInTheDocument();
    expect(screen.getByText("2 items share one password")).toBeInTheDocument();
    expect(screen.getByText("Weak passwords")).toBeInTheDocument();

    await user.click(screen.getByText("GitHub"));
    expect(onSelect).toHaveBeenCalledWith("a");
  });
});
```

- [ ] **Step 2: 跑測試確認失敗**

Run: `pnpm vitest run src/routes/Audit.test.tsx`
Expected: FAIL —— 無法解析 `./Audit`。

- [ ] **Step 3: 寫元件**

建立 `src/routes/Audit.tsx`:
```tsx
import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";
import type { AuditReport } from "../lib/types";
import { isAppError } from "../lib/types";
import { useT } from "../lib/i18n";

interface Props {
  onBack: () => void;
  onSelect: (id: string) => void;
}

export function Audit({ onBack, onSelect }: Props) {
  const t = useT();
  const [report, setReport] = useState<AuditReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  function load() {
    setLoading(true);
    setError(null);
    ipc
      .auditPasswords()
      .then(setReport)
      .catch((e) => setError(isAppError(e) ? e.message : String(e)))
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    load();
  }, []);

  const clean =
    report !== null && report.reused.length === 0 && report.weak.length === 0;

  return (
    <div className="screen audit-screen">
      <header className="audit-header">
        <button className="secondary" onClick={onBack}>{t("back")}</button>
        <h2>{t("audit_title")}</h2>
        <button className="secondary" onClick={load} disabled={loading}>
          {t("audit_rescan")}
        </button>
      </header>

      {error && <div className="error">{error}</div>}

      {loading ? (
        <p className="muted">{t("audit_loading")}</p>
      ) : report === null ? null : clean ? (
        <p className="muted audit-none">{t("audit_none")}</p>
      ) : (
        <>
          <p className="muted audit-summary">
            {t("audit_summary", {
              reused: String(report.reused.length),
              weak: String(report.weak.length),
            })}
          </p>

          {report.reused.length > 0 && (
            <section>
              <h3 className="audit-section">{t("audit_reused_section")}</h3>
              {report.reused.map((group, gi) => (
                <div key={gi} className="audit-group">
                  <div className="audit-group-head">
                    {t("audit_group_count", { count: String(group.items.length) })}
                  </div>
                  {group.items.map((it) => (
                    <button
                      key={it.id}
                      type="button"
                      className="audit-row"
                      onClick={() => onSelect(it.id)}
                    >
                      <div className="audit-row-main">
                        <div className="audit-row-title">
                          {it.site_name || t("list_unnamed")}
                        </div>
                        <div className="audit-row-sub">{it.username}</div>
                      </div>
                    </button>
                  ))}
                </div>
              ))}
            </section>
          )}

          {report.weak.length > 0 && (
            <section>
              <h3 className="audit-section">{t("audit_weak_section")}</h3>
              <div className="audit-group">
                {report.weak.map((w) => (
                  <button
                    key={w.item.id}
                    type="button"
                    className="audit-row"
                    onClick={() => onSelect(w.item.id)}
                  >
                    <div className="audit-row-main">
                      <div className="audit-row-title">
                        {w.item.site_name || t("list_unnamed")}
                      </div>
                      <div className="audit-row-sub">{w.item.username}</div>
                    </div>
                    <span className="audit-pill weak">{t("audit_weak_pill")}</span>
                  </button>
                ))}
              </div>
            </section>
          )}
        </>
      )}
    </div>
  );
}
```

- [ ] **Step 4: 加樣式**

附加到 `src/App.css` 檔尾:
```css
.audit-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 6px;
}
.audit-header h2 {
  flex: 1;
  margin: 0;
  font-size: 1.15rem;
}
.audit-summary {
  margin: 4px 0 16px;
}
.audit-section {
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--muted);
  margin: 20px 0 8px;
}
.audit-group {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
  margin-bottom: 10px;
}
.audit-group-head {
  font-size: 0.8rem;
  color: var(--muted);
  padding: 10px 14px 4px;
}
.audit-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  text-align: left;
  background: transparent;
  border: none;
  border-top: 1px solid var(--border);
  padding: 10px 14px;
  color: var(--text);
  cursor: pointer;
}
.audit-group-head + .audit-row {
  border-top: none;
}
.audit-row:hover {
  background: var(--surface-2);
}
.audit-row-main {
  flex: 1;
  min-width: 0;
}
.audit-row-sub {
  font-size: 0.8rem;
  color: var(--muted);
}
.audit-pill {
  font-size: 0.72rem;
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  color: var(--warn, #e0a800);
}
```
（若 `--warn` 變數不存在,上面的 fallback `#e0a800` 會生效;不需新增變數。)

- [ ] **Step 5: 跑測試確認通過**

Run: `pnpm vitest run src/routes/Audit.test.tsx`
Expected: PASS（2 個測試)。

- [ ] **Step 6: Commit**

```bash
git add src/routes/Audit.tsx src/routes/Audit.test.tsx src/App.css
git commit -m "feat(audit): audit screen component"
```

---

## Task 6：導航接線(App + List)+ 測試

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/routes/List.tsx`
- Test: `src/routes/List.test.tsx`、`src/App.test.tsx`

- [ ] **Step 1: 先更新 List 測試(失敗)**

在 `src/routes/List.test.tsx`:
（a)把 `renderList` helper 加入 `onAudit`:
```tsx
function renderList(props: Partial<Parameters<typeof List>[0]> = {}) {
  return renderWithI18n(
    <List
      refreshKey={0}
      onSelect={props.onSelect ?? vi.fn()}
      onLock={props.onLock ?? vi.fn()}
      onSettings={props.onSettings ?? vi.fn()}
      onAudit={props.onAudit ?? vi.fn()}
    />,
  );
}
```
（b)在 `describe("List", ...)` 內加一案:
```tsx
  it("opens the audit screen from the security-check button", async () => {
    const onAudit = vi.fn();
    const user = userEvent.setup();
    renderList({ onAudit });
    await waitForElementToBeRemoved(() => screen.queryByText("Loading…"));
    await user.click(screen.getByRole("button", { name: "Security check" }));
    expect(onAudit).toHaveBeenCalledTimes(1);
  });
```

- [ ] **Step 2: 跑 List 測試確認失敗**

Run: `pnpm vitest run src/routes/List.test.tsx`
Expected: 新案 FAIL（型別上 `onAudit` 未知 / 找不到按鈕);既有案仍 PASS。

- [ ] **Step 3: 在 List.tsx 加按鈕**

`src/routes/List.tsx`:
（a)`Props` 介面加 `onAudit: () => void;`,並在解構參數加入 `onAudit`:
```tsx
interface Props {
  onSelect: (id: string | "new") => void;
  onLock: () => void;
  onSettings: () => void;
  onAudit: () => void;
  refreshKey: number;
}

export function List({ onSelect, onLock, onSettings, onAudit, refreshKey }: Props) {
```
（b)在 `header-actions` 中、Add 按鈕之後加一顆:
```tsx
          <button className="secondary" onClick={onAudit}>{t("list_audit_btn")}</button>
```

- [ ] **Step 4: 跑 List 測試確認通過**

Run: `pnpm vitest run src/routes/List.test.tsx`
Expected: PASS（含新案)。

- [ ] **Step 5: App.tsx 導航**

`src/App.tsx`:
（a)頂部加 import:
```tsx
import { Audit } from "./routes/Audit";
```
（b)`Screen` 型別:加入 audit、並為 detail 加 `from`:
```tsx
type Screen =
  | { kind: "loading" }
  | { kind: "setup" }
  | { kind: "unlock"; reason?: "idle" | "manual" }
  | { kind: "list" }
  | { kind: "audit" }
  | { kind: "detail"; itemId: string | "new"; from?: "list" | "audit" }
  | { kind: "settings" };
```
（c)`list` case 的 `<List ... />` 加 prop:
```tsx
          onAudit={() => setScreen({ kind: "audit" })}
```
（d)把 `detail` case 換成(用區塊以宣告 `back`):
```tsx
    case "detail": {
      const back: Screen =
        screen.from === "audit" ? { kind: "audit" } : { kind: "list" };
      return (
        <ItemDetail
          itemId={screen.itemId}
          onClose={() => setScreen(back)}
          onSaved={() => {
            bumpList();
            setScreen(back);
          }}
          onDeleted={() => {
            bumpList();
            setScreen(back);
          }}
        />
      );
    }
```
（e)在 `settings` case 之前(或之後)加 `audit` case:
```tsx
    case "audit":
      return (
        <Audit
          onBack={() => setScreen({ kind: "list" })}
          onSelect={(id) => setScreen({ kind: "detail", itemId: id, from: "audit" })}
        />
      );
```

- [ ] **Step 6: 先更新 App 測試(失敗)**

在 `src/App.test.tsx`:
（a)ipc mock 物件加入兩個 fn:
```tsx
  auditPasswords: vi.fn(),
  getItem: vi.fn(),
```
（b)`beforeEach` 加預設:
```tsx
  ipc.auditPasswords.mockResolvedValue({ reused: [], weak: [] });
  ipc.getItem.mockResolvedValue({
    id: "x", site_name: "Forum", username: "nick", password: "weak",
    totp: null, url: "", notes: "", tags: [],
  });
```
（c)新增一個 describe / 測試:
```tsx
describe("App audit navigation", () => {
  it("goes list → audit → item → back to audit", async () => {
    ipc.auditPasswords.mockResolvedValue({
      reused: [],
      weak: [{ item: { id: "x", site_name: "Forum", username: "nick" }, score: 1 }],
    });
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "Keytainer", level: 1 });

    await user.click(screen.getByRole("button", { name: "Security check" }));
    expect(
      await screen.findByRole("heading", { name: "Security check" }),
    ).toBeInTheDocument();

    await user.click(await screen.findByText("Forum"));
    await screen.findByDisplayValue("Forum"); // ItemDetail loaded

    await user.click(screen.getByRole("button", { name: "← Back" }));
    expect(
      await screen.findByRole("heading", { name: "Security check" }),
    ).toBeInTheDocument();
  });
});
```
（d)在檔案頂部 import 加上 `userEvent`(若尚未 import):
```tsx
import userEvent from "@testing-library/user-event";
```

- [ ] **Step 7: 跑 App 測試確認通過**

Run: `pnpm vitest run src/App.test.tsx`
Expected: PASS（既有 + 新的導航案)。若新案因既有 App 測試設定而過於脆弱,改以單元層級(直接渲染 `Audit` / `List` 驗證 props)替代並在 commit 訊息註記;不要為了讓它過而放寬導航邏輯。

- [ ] **Step 8: Commit**

```bash
git add src/App.tsx src/routes/List.tsx src/routes/List.test.tsx src/App.test.tsx
git commit -m "feat(audit): wire audit screen into navigation"
```

---

## Task 7：文件 + 完整驗證

**Files:**
- Modify: `CHANGELOG.md`、`ROADMAP.md`、`README.md`

- [ ] **Step 1: 跑完整套件並記下數字**

Run: `pnpm test`
Expected: 全部 PASS。記下新總數(原 72,本計畫新增 Audit 2 + List 1 + App 1 = 4 → 預期 76;以實際印出為準)。
Run: `cd src-tauri && cargo test --lib`
Expected: 全部 PASS（原 60 + audit 5 = 65)。

- [ ] **Step 2: 更新 CHANGELOG**

在 `CHANGELOG.md` 的 `## [Unreleased]` → `### Added` 最上方加:
```markdown
- **Password audit.** A "Security check" screen (reachable from the vault
  list) flags reused passwords (two or more items sharing one password) and
  weak passwords (zxcvbn score below "fair"), computed entirely in the Rust
  backend so passwords never leave it. Each finding links to the item to fix.
```

- [ ] **Step 3: 更新 ROADMAP（繁中)**

在 `ROADMAP.md` 的「### 功能」中,把該項換成:
```markdown
- **重複/重用密碼偵測。** ✅ 從清單進入的「安全檢查」畫面,標示重用密碼
  (≥2 項目共用)與弱密碼(zxcvbn score < 2);完全在 Rust 後端計算,密碼
  不離開後端。每個發現都連到對應項目以便修正。
```

- [ ] **Step 4: 更新 README 測試數字**

在 `README.md`:把 Rust 測試數字行改為 Step 1 印出的後端數字(預期 65),前端測試數字行改為前端數字(預期 76)。

- [ ] **Step 5: 完整套件再跑一次**

Run: `pnpm test && (cd src-tauri && cargo test --lib)`
Expected: 全綠,數字與 README 一致。

- [ ] **Step 6: Commit**

```bash
git add CHANGELOG.md ROADMAP.md README.md
git commit -m "docs: record password audit; bump test counts"
```

---

## 自我審查筆記

- **Spec 覆蓋:** 後端 audit.rs 純函式 + 型別(§後端)→ Task 1;`audit_passwords` 命令(§後端)→ Task 2;前端型別/ipc(§前端)→ Task 3;i18n(§前端 i18n)→ Task 4;Audit.tsx + 樣式(§前端)→ Task 5;App/List 導航與 detail.from 修復循環(§導航)→ Task 6;測試(§測試)散落各 task;文件 → Task 7。全部覆蓋。
- **安全邊界:** 報告型別在 Rust 與 TS 皆只含 id/site_name/username,無密碼欄;audit 在後端執行。前端 fixtures 不含密碼。
- **既有測試安全:** List 既有測試透過 `renderList` helper 取得 props,Step 1 先把 `onAudit` 加進 helper,故既有案不受影響;App 既有測試新增 `auditPasswords`/`getItem` mock 不影響既有路由案(預設空報告)。
- **型別一致:** `AuditReport`/`ReuseGroup`/`WeakItem`/`AuditItemRef` 在 Rust(serde, snake_case 欄位 `site_name`)與 TS(同名 `site_name`)一致;`ipc.auditPasswords` 回傳 `AuditReport`,Audit.tsx 與 App.tsx 都用同一型別。
- **zxcvbn API 風險:** 已隔離在 `strength_score` 單一 helper,Step 4 的編譯/測試會立即暴露版本差異供調整。
</content>
