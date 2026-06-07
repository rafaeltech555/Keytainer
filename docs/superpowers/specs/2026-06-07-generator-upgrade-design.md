# 產生器升級 — 設計

日期:2026-06-07
狀態:已核准(待實作)

## 目標

把目前固定的密碼產生器(後端寫死字元集、前端寫死長度 20 + 符號開)升級為
可調的產生器:**passphrase 模式**(EFF 字典)、**排除易混淆字元**、以及
**UI 可調長度/選項**。實作 `ROADMAP.md` 的「產生器升級」。

控制項放在 ItemDetail 的內聯可展開面板,選項為元件本地、不持久化(Settings
不動)。版面對應 `docs/superpowers/mockups/generator-panel.html`。

## 後端

### 新模組 `src-tauri/src/generator.rs`

把產生邏輯從 `commands.rs` 抽出(目前的單一函式即將長大:字元模式 +
passphrase 模式 + 排除易混淆 + 字典)。`commands.rs` 的命令變薄包裝。

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenMode {
    Chars,
    Passphrase,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenOptions {
    pub mode: GenMode,
    // 字元模式
    pub length: usize,
    pub symbols: bool,
    pub avoid_ambiguous: bool,
    // passphrase 模式
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub number: bool,
}

/// 字元模式排除的易混淆字元。
const AMBIGUOUS: &[u8] = b"0O1lI";

pub fn generate(opts: &GenOptions) -> String { /* 見「行為」 */ }
```

**行為:**
- **字元模式(`Chars`):**
  - `length` clamp 至 `8..=128`。
  - 字母表:`a-z` + `A-Z` + `0-9`;`symbols` 時加 `!@#$%^&*()-_=+[]{};:,.?/`。
  - `avoid_ambiguous` 時從字母表移除 `AMBIGUOUS`(`0 O 1 l I`)。
  - 以 `rand::thread_rng()` 從字母表均勻取樣 `length` 個字元。
- **Passphrase 模式(`Passphrase`):**
  - `words` clamp 至 `3..=12`。
  - 從 EFF 字典均勻隨機抽 `words` 個詞(可重複,獨立抽樣)。
  - `capitalize` 時每個詞首字母大寫。
  - 以 `separator` 連接(UI 限定 `-`、`.`、`_`、空格;後端接受任意字串)。
  - `number` 時結尾直接接一位 `2..=9` 的數字(不加分隔符,對應 mockup 的
    `…Dragon7`;`2..=9` 避開 0/1 等易混淆數字)。
- 兩模式皆沿用既有做法:用後 `zeroize` 工作緩衝(明文必經 IPC 進 JS,那段
  無法 zeroize)。

### EFF 字典 `src-tauri/src/eff_large_wordlist.txt`

提交 EFF large wordlist(7776 詞,**僅單字、去掉骰號**),`include_str!` 後
以 `.lines()` 取用(7776 行 split 成本可忽略,或以 `OnceLock` 快取)。檔案
取得方式見實作計畫(由 EFF 下載後去骰號提交)。在 `generator.rs` 以註解標明
來源與授權:EFF Large Wordlist,CC-BY-3.0-US,https://www.eff.org/dice。
5 詞 ≈ 64 bits entropy。

### 命令(`src-tauri/src/commands.rs` + `lib.rs`)

把既有 `generate_password(length, include_symbols)` 換成:
```rust
#[tauri::command]
pub fn generate_password(opts: generator::GenOptions) -> String {
    generator::generate(&opts)
}
```
`lib.rs` 加 `pub mod generator;`(命令名不變,`invoke_handler` 不需改)。

## 前端

### `src/lib/types.ts`
```ts
export type GenMode = "chars" | "passphrase";
export interface GenOptions {
  mode: GenMode;
  length: number;
  symbols: boolean;
  avoid_ambiguous: boolean;
  words: number;
  separator: string;
  capitalize: boolean;
  number: boolean;
}
```

### `src/lib/ipc.ts`
把既有 `generatePassword(length, includeSymbols)` 換成:
```ts
generatePassword: (opts: GenOptions) =>
  invoke<string>("generate_password", { opts }),
```

### `src/routes/ItemDetail.tsx`
- 密碼欄的「Generate」按鈕改為一顆**展開鈕**(`gen_panel_toggle`),切換內聯
  產生器面板的顯示。
- 面板狀態(元件本地,不持久化)以 `GenOptions` 表示,預設:
  `{ mode: "chars", length: 20, symbols: true, avoid_ambiguous: false,
     words: 5, separator: "-", capitalize: true, number: true }`。
- 面板內容(對應 mockup):
  - 模式分段切換:隨機字元 / Passphrase。
  - 隨機:長度滑桿(8–64)、符號開關、排除易混淆開關。
  - Passphrase:詞數滑桿(3–12)、分隔符選單(`-`/`.`/`_`/空格)、首字母大寫
    開關、加數字開關。
  - 「產生」按鈕 → `await ipc.generatePassword(opts)`,`patch("password", pw)`、
    `setConfirmWeak(false)`、`setShowPw(true)`。
- 移除舊的固定 `generatePassword(20, true)` 呼叫。

### i18n(`src/lib/i18n.tsx`,EN + zh-TW)

| Key | EN | 繁中 |
|-----|----|----|
| `gen_panel_toggle` | Generator | 產生器 |
| `gen_mode_random` | Random | 隨機字元 |
| `gen_mode_passphrase` | Passphrase | Passphrase |
| `gen_length` | Length | 長度 |
| `gen_symbols` | Symbols | 符號 |
| `gen_avoid_ambiguous` | Avoid ambiguous (0 O 1 l I) | 排除易混淆 (0 O 1 l I) |
| `gen_words` | Words | 詞數 |
| `gen_separator` | Separator | 分隔符 |
| `gen_sep_space` | Space | 空格 |
| `gen_capitalize` | Capitalize | 首字母大寫 |
| `gen_number` | Add a number | 加數字 |
| `gen_generate` | Generate | 產生 |

### 樣式（`src/App.css`)
附加 `gen-panel` 相關規則(分段切換、列、滑桿、開關),沿用既有 palette。

## 測試

**後端 — `src-tauri/src/generator.rs`(`#[cfg(test)]`):**
- 字元模式:回傳長度等於 `length`;`length` 被 clamp(如傳 4 → 8、傳 999 → 128)。
- 字元模式 `symbols=false`:輸出僅含英數。
- 字元模式 `avoid_ambiguous=true`:輸出不含 `0 O 1 l I` 任一字元。
- passphrase:`words=5` 產生 5 個詞(依分隔符 split);`separator="-"`;
  `capitalize=true` 時每詞首字母大寫;`number=true` 時結尾為一位 `2..=9` 數字。
- 字典已載入:行數為 7776,且首/末詞符合 EFF(`abacus` … `zoom`)。
- 隨機性 sanity:相同選項連續產生兩次不相等(極高機率)。

**前端 — `src/routes/ItemDetail.test.tsx`:**
- 既有的「fills the password field from the generator」測試改用新簽章:
  展開面板 → 按「產生」→ `ipc.generatePassword` 以一個 `GenOptions` 物件被呼叫
  (預設 `mode:"chars"`),結果填回欄位。
- 切到 Passphrase 模式、調詞數後按產生 → `generatePassword` 帶 `mode:"passphrase"`
  與調整後的 `words`。
- 隨機模式調長度後按產生 → opts 帶調整後的 `length`。
- mock `ipc.generatePassword`。

## 不做(YAGNI)

- Settings 持久化產生器偏好。
- 自訂字典匯入。
- 每字元類別最少數量保證(如「至少 2 個數字」)。
- entropy / 強度即時數字(已有強度計顯示)。
- 獨立的產生器畫面。
- passphrase 模式的排除易混淆開關(passphrase 由詞構成;僅其結尾數字用
  `2..=9` 避開易混淆數字)。

## 完成時要更新的文件

- `CHANGELOG.md`(Unreleased → Added,英文)。
- `ROADMAP.md`(將產生器升級標為完成)。
- `README.md`(若測試數字那行需更新)。
</content>
