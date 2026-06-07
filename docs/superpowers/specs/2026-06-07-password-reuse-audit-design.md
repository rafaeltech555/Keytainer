# 密碼重用與弱密碼 Audit — 設計

日期:2026-06-07
狀態:已核准(待實作)

## 目標

幫使用者找出並修正最弱的環節:被多個項目重用的密碼,以及單純偏弱的密碼。
在一個從金庫清單進入的專屬 audit 畫面中呈現它們,每個發現都連到對應項目以
便修正。本設計實作 `ROADMAP.md` 的「重複/重用密碼偵測與基本 audit 視圖」。

範圍**僅限重用 + 弱**偵測。密碼歷史是另一個獨立、之後的 cycle(各自有
spec → plan → 實作)。

## 安全邊界(決定整體架構)

`list_items` 回傳的 `ItemSummary` 刻意**不含密碼** —— 密碼留在 Rust 後端,
只透過 `get_item`(單一項目)或 `copy_password`(複製到剪貼簿)離開後端。
若把所有密碼一次載入前端分析,等於把每個密鑰同時放進 JS heap,本設計避免
這件事。

因此 audit **完全在 Rust 後端執行**。它在內部比對密碼,只回傳「哪些項目」
有問題 —— 報告**永不**包含密碼值。

## 做法

由一個純後端函式分析已解鎖的金庫並回傳結構化報告;一個 Tauri 命令對外
暴露它;前端一個專屬畫面負責渲染。

- **重用:** 依完全相同的密碼字串(區分大小寫、逐位元組)分組。跳過空密碼。
  任何 ≥2 個項目的組即為一個 reuse group。
- **弱:** 每個非空密碼以 Rust `zxcvbn` crate 評分;`score < WEAK_SCORE`
  (= 2)即標記為弱。此門檻對齊前端強度計的「fair」界限
  (`MIN_MASTER_SCORE = 2`),讓兩個功能對「弱」的定義一致。

(已否決:在前端評分 —— 需把所有密碼送進 JS,破壞安全邊界。已否決:Rust
輕量 heuristic —— 使用者選擇 zxcvbn 以與強度計一致。)

## 後端

### 新增 crate 依賴

在 `src-tauri/Cargo.toml` 加入 `zxcvbn`(Rust)作為一般(非 optional)依賴。

### `src-tauri/src/audit.rs`

一個對金庫運作的純函式,加上報告型別。無 I/O、無 Tauri —— 可直接單元測試。

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

pub fn audit(vault: &Vault) -> AuditReport { /* 見「行為」 */ }
```

**行為:**
- 走訪 `vault.items`,略過任何 `password` 為空的項目。
- **重用:** 建立 `password -> Vec<AuditItemRef>` 的對應;每個有 ≥2 個 ref
  的條目產生一個 `ReuseGroup`。組的順序:依第一個項目的 `site_name`
  (不分大小寫);組內項目維持金庫順序。(輸出具決定性,利於測試/UI 穩定。)
- **弱:** 每個非空密碼項目計算 zxcvbn 分數;若 `score < WEAK_SCORE`,加入
  一個 `WeakItem`。弱項目依 `site_name`(不分大小寫)排序。
- 共用的密碼值永不存進報告。

> zxcvbn crate 註記:呼叫該 crate 的評分入口、讀取其 0–4 分數。**不**把
> site_name/username 當作 user-inputs 傳入(與前端強度計一致,前端未傳任何
> input)。確切的 crate API(回傳型別/方法名)於實作時對照解析到的版本釘定。

### `src-tauri/src/commands.rs`

```rust
#[tauri::command]
pub fn audit_passwords(state: State<'_, AppState>) -> AppResult<audit::AuditReport> {
    state.with_session(|s| Ok(audit::audit(&s.vault)))
}
```

在 Tauri 的 `invoke_handler` 註冊此命令,並於 `lib.rs` 加入 `pub mod audit;`。
此命令**按需**呼叫(開啟 audit 畫面時),而非每次載入清單時。

## 前端

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

- 掛載時呼叫 `ipc.auditPasswords()`;先顯示載入狀態,再顯示報告。
- 標頭:返回按鈕 + 標題(`audit_title`)+ 重新掃描按鈕(重新抓取)。
- 摘要列:reuse 組數與 weak 數(`audit_summary`)。
- **重用**區:每個 `ReuseGroup` 一張卡;一個「♻ N 個項目共用一組密碼」標籤;
  每個項目是可點的列(site_name + username),點擊呼叫 `onSelect(item.id)`。
- **弱**區:可點的列,每列一個 weak pill。
- 空狀態(沒有重用也沒有弱):`audit_none`(「沒有發現問題 ✓」)。
- 永不渲染密碼值。
- 錯誤沿用既有的 `isAppError` 模式處理。

### 導航 — `src/App.tsx`

- 擴充 `Screen`:加入 `{ kind: "audit" }`,並為 detail 加上來源:
  `{ kind: "detail"; itemId: string | "new"; from?: "list" | "audit" }`。
- `List` 增加 `onAudit` prop;其標頭加一顆「安全檢查」按鈕(僅文字、不帶
  數字,讓清單永不觸發 audit),點擊設為 `{ kind: "audit" }`。
- `Audit` 畫面:`onBack` → `{ kind: "list" }`;`onSelect(id)` →
  `{ kind: "detail", itemId: id, from: "audit" }`。
- `ItemDetail` 的 `onClose` / `onSaved` / `onDeleted` 回到來源:若
  `screen.from === "audit"`,回到 `{ kind: "audit" }`(會重新抓取,已修的
  項目自動消失);否則維持現狀回到 `{ kind: "list" }`。`onSaved`/`onDeleted`
  仍呼叫 `bumpList()`。

### `src/routes/List.tsx`

在既有標頭操作區加上「安全檢查」按鈕,接到新的 `onAudit` prop。List 其餘
行為不變。

### i18n（`src/lib/i18n.tsx`,EN + zh-TW)

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

（`audit_summary` / `audit_group_count` 使用 `t` 既有的 `{name}` 佔位機制。)

### 樣式（`src/App.css`)

附加一個 `audit-*` 區塊(reuse 組卡片、可點的發現列、weak/reuse pill)。重用
既有的 palette 變數;元件內不寫死像素值。外觀對應
`docs/superpowers/mockups/audit-A-dedicated-screen.html`。

## 測試

**後端 — `src-tauri/src/audit.rs`（`#[cfg(test)]`):**
- 三個項目共用一組密碼 → 恰好一個 `ReuseGroup`、含三個 ref(id/site_name
  正確)。
- 兩組不同的共用密碼 → 兩個獨立的組;每個都唯一的密碼 → 零組。
- 常見密碼(`"password"`)被標記為弱;強 passphrase 不被標。
- 空密碼項目完全略過(既非重用也非弱)。
- 報告不含任何密碼字串(結構性 —— `AuditReport` 沒有密碼欄;以資料驗證,
  例如重用配對也只暴露 id/site_name/username)。
- 輸出排序具決定性(依 site_name)。

**前端:**
- `src/routes/Audit.test.tsx` —— mock `ipc.auditPasswords`:
  - 以一份 fixture 報告渲染一個 reuse 組與一個 weak 清單;點擊發現會以項目
    id 呼叫 `onSelect`;
  - 空報告渲染 `audit_none`;
  - 永不顯示密碼(依設計,fixture 本就不含密碼)。
- `src/routes/List.test.tsx`(新增)—— 「安全檢查」按鈕呼叫 `onAudit`。
- `src/App.test.tsx`(若不過度牽動既有設定則新增)—— 導航
  list → audit → detail（from audit)→ 返回會回到 audit。(若此案對既有
  App 測試設定過於脆弱,改以 props 層級驗證導航並註記。)

## 不做（YAGNI）

- 清單列上的行內 reuse/weak 徽章(取向 B)。
- 自動/背景掃描或清單層級的問題計數。
- 「太久未更改」(stale password)偵測。
- audit 內一鍵重設/輪替密碼。
- 在 UI 顯示弱密碼的數值分數(僅二元 weak/not-weak)。
- 密碼歷史(另一個之後的 cycle)。

## 完成時要更新的文件

- `CHANGELOG.md`(Unreleased → Added)。
- `ROADMAP.md`(將重用/audit 那項標為完成;密碼歷史留作下一項)。
- `README.md`(若測試數字那行需更新)。
</content>
