# 密碼歷史 實作計畫

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 每筆項目保留最近 10 個用過的密碼,在 ItemDetail 可檢視/複製/還原;擷取在後端純函式,向後相容免遷移。

**Architecture:** `VaultItem` 加 `password_history` 欄位(serde default);`crud::update_item` 在密碼變更時把舊密碼 prepend 進歷史(上限 10);`get_item` 帶出歷史;新命令 `copy_history_password` 走既有剪貼簿自動清除;ItemDetail 顯示歷史區。

**Tech Stack:** Rust + Tauri 2、serde、zeroize、React + TypeScript、Vitest + Testing Library。

**Spec:** `docs/superpowers/specs/2026-06-07-password-history-design.md`

---

## 檔案結構

- 改 `src-tauri/src/vault/mod.rs` —— `PasswordHistoryEntry` 型別 + `VaultItem.password_history` 欄位。
- 改 5 處 `VaultItem` 建構點(補新欄位):`commands.rs` 的 `into_vault_item`、以及 `audit.rs` / `backup.rs` / `vault/store.rs` / `vault/crud.rs` 的測試 helper。
- 改 `src-tauri/src/vault/crud.rs` —— `MAX_HISTORY` + `update_item` 擷取邏輯 + 測試。
- 改 `src-tauri/src/commands.rs` + `lib.rs` —— `copy_history_password` 命令 + 註冊。
- 改 `src/lib/types.ts`、`src/lib/ipc.ts` —— 型別與 binding。
- 改 `src/lib/i18n.tsx` —— `detail_history_*` 字串(EN + zh-TW)。
- 改 `src/routes/ItemDetail.tsx` + `src/routes/ItemDetail.test.tsx` —— 歷史區 UI。
- 改 `src/App.css` —— 樣式。
- 文件:`CHANGELOG.md`、`ROADMAP.md`、`README.md`。

指令從 repo 根目錄執行。後端測試從 `src-tauri/` 跑。所有 commit 訊息結尾加:
```
Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 1：資料模型(VaultItem.password_history)

**Files:**
- Modify: `src-tauri/src/vault/mod.rs`
- Modify: `src-tauri/src/commands.rs`、`src-tauri/src/audit.rs`、`src-tauri/src/backup.rs`、`src-tauri/src/vault/store.rs`、`src-tauri/src/vault/crud.rs`(補建構欄位)

- [ ] **Step 1: 寫失敗測試(向後相容)**

在 `src-tauri/src/vault/crud.rs` 的 `#[cfg(test)] mod tests` 內加一個測試:
```rust
    #[test]
    fn item_without_history_field_deserializes_to_empty() {
        let json = r#"{"id":"00000000-0000-0000-0000-000000000000","site_name":"S","username":"u","password":"p","tags":[],"created_at":0,"updated_at":0}"#;
        let item: VaultItem = serde_json::from_str(json).unwrap();
        assert!(item.password_history.is_empty());
    }
```
（`VaultItem` 已在該測試模組 `use` 進來;`serde_json` 是既有依賴。)

- [ ] **Step 2: 跑測試確認失敗**

Run: `cd src-tauri && cargo test --lib vault::crud`
Expected: 編譯錯誤 —— `VaultItem` 無 `password_history` 欄位。FAIL。

- [ ] **Step 3: 新增型別與欄位**

在 `src-tauri/src/vault/mod.rs`,於 `VaultItem` 結構定義之後加上新型別:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct PasswordHistoryEntry {
    pub password: String,
    #[zeroize(skip)]
    pub changed_at: i64,
}
```
並在 `VaultItem` 結構中、`tags` 欄位之後、`created_at` 之前加:
```rust
    #[serde(default)]
    pub password_history: Vec<PasswordHistoryEntry>,
```
（`Serialize`/`Deserialize`/`Zeroize`/`ZeroizeOnDrop`/`Vec` 等已在該檔 import。`#[serde(default)]` 讓舊 JSON 解出為空 Vec。)

- [ ] **Step 4: 補齊 5 處 VaultItem 建構點**

每處在建構 `VaultItem { ... }` 時補上一行 `password_history`:

1. `src-tauri/src/commands.rs` 的 `into_vault_item`(`VaultItem { ... created_at: 0, updated_at: 0 }`)→ 在 `tags` 之後加:
   ```rust
            password_history: Vec::new(),
   ```
2. `src-tauri/src/vault/crud.rs` 測試的 `fn item(...)` → 加:
   ```rust
            password_history: vec![],
   ```
3. `src-tauri/src/audit.rs` 測試的 `fn item(...)` → 加 `password_history: vec![],`
4. `src-tauri/src/backup.rs` 測試中的 `VaultItem { ... }` → 加 `password_history: vec![],`
5. `src-tauri/src/vault/store.rs` 測試的 `fn sample_item()` → 加 `password_history: vec![],`

- [ ] **Step 5: 跑測試確認通過**

Run: `cd src-tauri && cargo test --lib`
Expected: 全部 PASS(既有 65 + 新的向後相容測試 = 66)。`cargo build --lib` warning-free。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/vault/mod.rs src-tauri/src/commands.rs src-tauri/src/audit.rs src-tauri/src/backup.rs src-tauri/src/vault/store.rs src-tauri/src/vault/crud.rs
git commit -m "feat(history): add password_history field to VaultItem"
```

---

## Task 2：擷取邏輯(crud::update_item)

**Files:**
- Modify: `src-tauri/src/vault/crud.rs`

- [ ] **Step 1: 寫失敗測試**

在 `src-tauri/src/vault/crud.rs` 的測試模組加入:
```rust
    #[test]
    fn records_old_password_on_change() {
        let mut v = Vault::default();
        let id = add_item(&mut v, item("GitHub", "alice"));
        let mut next = v.items[0].clone();
        next.password = "newpw".into();
        update_item(&mut v, next).unwrap();
        assert_eq!(v.items[0].password_history.len(), 1);
        assert_eq!(v.items[0].password_history[0].password, "pw");
        assert!(v.items[0].password_history[0].changed_at > 0);
        let _ = id;
    }

    #[test]
    fn unchanged_password_records_nothing() {
        let mut v = Vault::default();
        add_item(&mut v, item("GitHub", "alice"));
        let mut next = v.items[0].clone();
        next.username = "bob".into(); // password unchanged
        update_item(&mut v, next).unwrap();
        assert!(v.items[0].password_history.is_empty());
    }

    #[test]
    fn history_is_newest_first() {
        let mut v = Vault::default();
        add_item(&mut v, item("GitHub", "alice")); // pw = "pw"
        let mut a = v.items[0].clone();
        a.password = "B".into();
        update_item(&mut v, a).unwrap(); // records "pw"
        let mut b = v.items[0].clone();
        b.password = "C".into();
        update_item(&mut v, b).unwrap(); // records "B"
        let hist: Vec<&str> = v.items[0].password_history.iter().map(|e| e.password.as_str()).collect();
        assert_eq!(hist, vec!["B", "pw"]);
    }

    #[test]
    fn history_is_capped_at_max() {
        let mut v = Vault::default();
        add_item(&mut v, item("GitHub", "alice"));
        for n in 0..MAX_HISTORY + 5 {
            let mut next = v.items[0].clone();
            next.password = format!("pw{n}");
            update_item(&mut v, next).unwrap();
        }
        assert_eq!(v.items[0].password_history.len(), MAX_HISTORY);
    }

    #[test]
    fn empty_old_password_is_not_recorded() {
        let mut v = Vault::default();
        let mut it = item("GitHub", "alice");
        it.password = "".into();
        add_item(&mut v, it);
        let mut next = v.items[0].clone();
        next.password = "firstpw".into();
        update_item(&mut v, next).unwrap();
        assert!(v.items[0].password_history.is_empty());
    }
```
（`item("GitHub","alice")` 既有 helper 的密碼是 `"pw"`;`MAX_HISTORY` 由 Step 3 引入,測試的 `use super::*;` 會帶入。)

- [ ] **Step 2: 跑測試確認失敗**

Run: `cd src-tauri && cargo test --lib vault::crud`
Expected: 編譯錯誤(`MAX_HISTORY` 未定義)或斷言失敗(歷史未記錄)。FAIL。

- [ ] **Step 3: 實作擷取**

在 `src-tauri/src/vault/crud.rs` 頂部(`now_unix` 附近)加常數:
```rust
/// Max number of previous passwords retained per item.
pub const MAX_HISTORY: usize = 10;
```
把 `update_item` 換成:
```rust
pub fn update_item(vault: &mut Vault, updated: VaultItem) -> AppResult<()> {
    let id = updated.id;
    let pos = vault
        .items
        .iter()
        .position(|i| i.id == id)
        .ok_or(AppError::ItemNotFound(id))?;

    let created_at = vault.items[pos].created_at;

    // Record the old password before replacing, if it actually changed and
    // was non-empty. Newest first, capped at MAX_HISTORY.
    let mut history = vault.items[pos].password_history.clone();
    let old_password = vault.items[pos].password.clone();
    if !old_password.is_empty() && old_password != updated.password {
        history.insert(
            0,
            PasswordHistoryEntry { password: old_password, changed_at: now_unix() },
        );
        history.truncate(MAX_HISTORY);
    }

    let mut next = updated;
    next.created_at = created_at;
    next.updated_at = now_unix();
    next.password_history = history;
    register_tags(vault, &next.tags);
    vault.items[pos] = next;
    Ok(())
}
```
並確認該檔頂部 `use super::{Vault, VaultItem};` 需擴充為包含 `PasswordHistoryEntry`(改為 `use super::{PasswordHistoryEntry, Vault, VaultItem};`)。

- [ ] **Step 4: 跑測試確認通過**

Run: `cd src-tauri && cargo test --lib vault::crud`
Expected: PASS(含 5 個新擷取測試 + Task 1 的向後相容測試)。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault/crud.rs
git commit -m "feat(history): capture old password on change (cap 10)"
```

---

## Task 3：copy_history_password 命令

**Files:**
- Modify: `src-tauri/src/commands.rs`、`src-tauri/src/lib.rs`

- [ ] **Step 1: 加入命令**

在 `src-tauri/src/commands.rs`,於既有 `copy_password` 命令之後加(鏡像其結構,但取歷史 index):
```rust
#[tauri::command]
pub fn copy_history_password(
    id: Uuid,
    index: usize,
    state: State<'_, AppState>,
    clipboard: State<'_, ClipboardState>,
) -> AppResult<()> {
    let secret = state.with_session(|s| {
        s.vault
            .items
            .iter()
            .find(|i| i.id == id)
            .and_then(|i| i.password_history.get(index))
            .map(|e| e.password.clone())
            .ok_or(AppError::ItemNotFound(id))
    })?;
    let cfg = settings::load();
    clipboard.write_with_auto_clear(secret, Duration::from_secs(cfg.clipboard_clear_seconds))
}
```
（`Uuid`、`State`、`AppState`、`ClipboardState`、`AppResult`、`AppError`、`settings`、`Duration` 都是該檔既有 import —— 與 `copy_password` 用到的相同。)

- [ ] **Step 2: 註冊命令**

在 `src-tauri/src/lib.rs` 的 `tauri::generate_handler![ ... ]` 中,於 `commands::copy_password,` 之後加:
```rust
            commands::copy_history_password,
```

- [ ] **Step 3: 編譯 + 測試**

Run: `cd src-tauri && cargo test --lib`
Expected: 全部 PASS,`cargo build --lib` warning-free。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(history): copy_history_password command (auto-clear)"
```

---

## Task 4：前端型別 + ipc binding

**Files:**
- Modify: `src/lib/types.ts`、`src/lib/ipc.ts`

- [ ] **Step 1: 加型別**

在 `src/lib/types.ts`:加入 `PasswordHistoryEntry`,並在 `VaultItem` 介面加一個**可選**欄位(讓既有 mock 不需補欄位):
```ts
export interface PasswordHistoryEntry {
  password: string;
  changed_at: number;
}
```
在 `VaultItem` 介面內(例如 `tags` 之後)加:
```ts
  password_history?: PasswordHistoryEntry[];
```

- [ ] **Step 2: 加 ipc binding**

在 `src/lib/ipc.ts`,於 `copyPassword` 之後加:
```ts
  copyHistoryPassword: (id: string, index: number) =>
    invoke<void>("copy_history_password", { id, index }),
```

- [ ] **Step 3: 型別檢查**

Run: `pnpm tsc --noEmit`
Expected: 無錯誤。

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/ipc.ts
git commit -m "feat(history): frontend types and ipc binding"
```

---

## Task 5：i18n 字串

**Files:**
- Modify: `src/lib/i18n.tsx`（`en` 與 `zh-TW`)

- [ ] **Step 1: 在 `en` 字典加入**(放在 detail_ 相關鍵附近,新增 `// Password history` 小節):
```ts
  // Password history
  detail_history_section: "Password history",
  detail_history_show: "Show passwords",
  detail_history_hide: "Hide passwords",
  detail_history_copy: "Copy",
  detail_history_copied: "Copied",
  detail_history_restore: "Use again",
```

- [ ] **Step 2: 在 `zh-TW` 字典加入對應**:
```ts
  // Password history
  detail_history_section: "密碼歷史",
  detail_history_show: "顯示密碼",
  detail_history_hide: "隱藏密碼",
  detail_history_copy: "複製",
  detail_history_copied: "已複製",
  detail_history_restore: "重新使用",
```

- [ ] **Step 3: 驗證 parity**

Run: `pnpm vitest run src/lib/i18n.test.tsx`
Expected: PASS。

- [ ] **Step 4: Commit**

```bash
git add src/lib/i18n.tsx
git commit -m "i18n: add password history strings (EN + zh-TW)"
```

---

## Task 6：ItemDetail 歷史區 + 測試

**Files:**
- Modify: `src/routes/ItemDetail.tsx`、`src/routes/ItemDetail.test.tsx`
- Modify: `src/App.css`

- [ ] **Step 1: 寫失敗測試**

在 `src/routes/ItemDetail.test.tsx`:
（a)`ipc` 的 `vi.hoisted` mock 物件加入 `copyHistoryPassword: vi.fn(),`。
（b)加兩個測試:
```tsx
  it("shows password history with reveal, copy, and restore", async () => {
    ipc.getItem.mockResolvedValue(
      vaultItem({
        password_history: [
          { password: "oldpw1", changed_at: 1700000000 },
          { password: "oldpw2", changed_at: 1690000000 },
        ],
      }),
    );
    const user = userEvent.setup();
    renderDetail("1");

    expect(await screen.findByText("Password history")).toBeInTheDocument();
    expect(screen.queryByText("oldpw1")).not.toBeInTheDocument(); // masked

    await user.click(screen.getByRole("button", { name: "Show passwords" }));
    expect(screen.getByText("oldpw1")).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Copy" })[0]);
    expect(ipc.copyHistoryPassword).toHaveBeenCalledWith("1", 0);

    await user.click(screen.getAllByRole("button", { name: "Use again" })[1]);
    expect(screen.getByLabelText("Password")).toHaveValue("oldpw2");
  });

  it("hides the history section when there is none", async () => {
    ipc.getItem.mockResolvedValue(vaultItem({ password_history: [] }));
    renderDetail("1");
    await screen.findByDisplayValue("GitHub");
    expect(screen.queryByText("Password history")).not.toBeInTheDocument();
  });
```

- [ ] **Step 2: 跑測試確認失敗**

Run: `pnpm vitest run src/routes/ItemDetail.test.tsx`
Expected: 新兩案 FAIL(無歷史區);既有案 PASS。

- [ ] **Step 3: 實作歷史區**

在 `src/routes/ItemDetail.tsx`:
（a)型別 import 加 `PasswordHistoryEntry`(該檔已從 `../lib/types` import 型別,擴充該行)。
（b)在其他 `useState` 旁加狀態,並在既有的 `pwCopiedTimer` 旁加一個 timer ref:
```tsx
  const [history, setHistory] = useState<PasswordHistoryEntry[]>([]);
  const [showHistory, setShowHistory] = useState(false);
  const [copiedIdx, setCopiedIdx] = useState<number | null>(null);
  const histCopiedTimer = useRef<number | null>(null);
```
（c)在載入項目的 `.then((it) => { ... })` 區塊(`setForm({...}); setTagsInput(...)` 之後)加:
```tsx
        setHistory(it.password_history ?? []);
```
（d)加兩個處理函式(放在 `copyPassword` 附近):
```tsx
  async function copyHistory(index: number) {
    if (!savedItemId) return;
    try {
      await ipc.copyHistoryPassword(savedItemId, index);
      setCopiedIdx(index);
      if (histCopiedTimer.current) window.clearTimeout(histCopiedTimer.current);
      histCopiedTimer.current = window.setTimeout(() => setCopiedIdx(null), 1500);
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    }
  }

  function restoreHistory(entry: PasswordHistoryEntry) {
    patch("password", entry.password);
    setConfirmWeak(false);
    setShowPw(true);
  }
```
（e)在密碼 `<label>` 之後(URL `<label>` 之前)插入歷史區:
```tsx
        {history.length > 0 && (
          <div className="history-block">
            <div className="history-head">
              <span>{t("detail_history_section")}</span>
              <button
                type="button"
                className="secondary"
                onClick={() => setShowHistory((s) => !s)}
              >
                {showHistory ? t("detail_history_hide") : t("detail_history_show")}
              </button>
            </div>
            <ul className="history-list">
              {history.map((entry, i) => (
                <li key={i} className="history-row">
                  <div className="history-main">
                    <span className="history-pw">
                      {showHistory ? entry.password : "••••••••"}
                    </span>
                    <span className="history-date">
                      {new Date(entry.changed_at * 1000).toLocaleDateString()}
                    </span>
                  </div>
                  <div className="history-actions">
                    <button type="button" onClick={() => copyHistory(i)}>
                      {copiedIdx === i ? t("detail_history_copied") : t("detail_history_copy")}
                    </button>
                    <button type="button" onClick={() => restoreHistory(entry)}>
                      {t("detail_history_restore")}
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        )}
```

- [ ] **Step 4: 加樣式**

附加到 `src/App.css` 檔尾:
```css
.history-block {
  margin: 4px 0 12px;
}
.history-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.8rem;
  color: var(--muted);
  margin-bottom: 6px;
}
.history-list {
  list-style: none;
  margin: 0;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
}
.history-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
}
.history-row + .history-row {
  border-top: 1px solid var(--border);
}
.history-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.history-pw {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  word-break: break-all;
}
.history-date {
  font-size: 0.75rem;
  color: var(--muted);
}
.history-actions {
  display: flex;
  gap: 6px;
}
```

- [ ] **Step 5: 跑測試確認通過**

Run: `pnpm vitest run src/routes/ItemDetail.test.tsx`
Expected: PASS(既有 + 新兩案)。

- [ ] **Step 6: Commit**

```bash
git add src/routes/ItemDetail.tsx src/routes/ItemDetail.test.tsx src/App.css
git commit -m "feat(history): password history section in ItemDetail"
```

---

## Task 7：文件 + 完整驗證

**Files:**
- Modify: `CHANGELOG.md`、`ROADMAP.md`、`README.md`

- [ ] **Step 1: 跑完整套件並記下數字**

Run: `pnpm test`
Expected: 全部 PASS。記下新總數(原 76,本計畫新增 ItemDetail 2 = 78;以實際為準)。
Run: `cd src-tauri && cargo test --lib`
Expected: 全部 PASS(原 65 + Task 1 的 1 + Task 2 的 5 = 71)。

- [ ] **Step 2: 更新 CHANGELOG**(英文,`## [Unreleased]` → `### Added` 最上方):
```markdown
- **Password history.** Each item now keeps its last 10 previous passwords,
  captured automatically when a password changes. View, copy (with clipboard
  auto-clear), or restore a previous password from the item screen. History
  travels inside the encrypted vault and its backups.
```

- [ ] **Step 3: 更新 ROADMAP（繁中)** —— 把「### 功能」中的密碼歷史那項換成:
```markdown
- **密碼歷史。** ✅ 每筆項目保留最近 10 個用過的密碼(密碼變更時自動記錄);
  在項目畫面可檢視、複製(剪貼簿自動清除)或還原。歷史隨加密 vault 與其備份
  一併保存。
```

- [ ] **Step 4: 更新 README 測試數字** —— Rust 測試數字行改為 Step 1 的後端數字(預期 71),前端測試數字行改為前端數字(預期 78)。

- [ ] **Step 5: 完整套件再跑一次**

Run: `pnpm test && (cd src-tauri && cargo test --lib)`
Expected: 全綠,數字與 README 一致。

- [ ] **Step 6: Commit**

```bash
git add CHANGELOG.md ROADMAP.md README.md
git commit -m "docs: record password history; bump test counts"
```

---

## 自我審查筆記

- **Spec 覆蓋:** 資料模型(§資料模型)→ Task 1;擷取(§擷取邏輯)→ Task 2;命令(§命令)→ Task 3;前端型別/ipc(§前端)→ Task 4;i18n → Task 5;ItemDetail UI(§前端 ItemDetail)→ Task 6;文件 → Task 7。全部覆蓋。
- **向後相容:** `#[serde(default)]` + Task 1 的 round-trip 測試;TS 欄位可選,既有 mock 不破。
- **5 處建構點:** Task 1 Step 4 明列 commands/crud/audit/backup/store —— 漏任一處都會編譯失敗,Step 5 的 `cargo test` 會抓到。
- **型別一致:** `PasswordHistoryEntry`(Rust serde 欄位 `password`/`changed_at` snake_case)對應 TS 同名;`MAX_HISTORY` 在 crud,測試 `use super::*` 取得;ItemInput 不含歷史(後端維護)。
- **顯示/隱藏歧義:** 歷史區用 `detail_history_show`/`detail_history_hide`(「Show passwords」),與密碼欄的 `detail_show`/`detail_hide`(「Show」)區隔,測試 `getByRole(name:"Show passwords")` 不會撞到密碼欄的「Show」。
- **複製安全:** `copy_history_password` 鏡像 `copy_password`,走 `ClipboardState` 自動清除;密碼明文雖已隨 `get_item` 在前端(供 reveal),複製仍走後端自動清除路徑維持一致。
</content>
