# 產生器升級 實作計畫

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把固定的密碼產生器升級為可調的:passphrase 模式(EFF 字典)、排除易混淆字元、ItemDetail 內聯產生器面板可調長度/選項。

**Architecture:** 後端產生邏輯抽到 `generator.rs` 模組,命令改吃 `GenOptions` struct;EFF large wordlist 以 `include_str!` 內嵌;前端 ItemDetail 加可展開面板,以 `GenOptions` 呼叫。

**Tech Stack:** Rust + Tauri 2、rand、serde、React + TypeScript、Vitest + Testing Library。

**Spec:** `docs/superpowers/specs/2026-06-07-generator-upgrade-design.md`

---

## 檔案結構

- 新增 `src-tauri/src/eff_large_wordlist.txt` —— EFF 字典(7776 詞,僅單字)。
- 新增 `src-tauri/src/generator.rs` —— `GenMode`/`GenOptions`/`generate` + `#[cfg(test)]`。
- 改 `src-tauri/src/lib.rs` —— 加 `pub mod generator;`。
- 改 `src-tauri/src/commands.rs` —— `generate_password` 改吃 `GenOptions`,移除舊內聯邏輯。
- 改 `src/lib/types.ts`、`src/lib/ipc.ts` —— `GenMode`/`GenOptions` + ipc 簽章。
- 改 `src/lib/i18n.tsx` —— `gen_*` 字串(EN + zh-TW)。
- 改 `src/routes/ItemDetail.tsx` + `src/routes/ItemDetail.test.tsx` —— 面板 UI。
- 改 `src/App.css` —— 面板樣式。
- 文件:`CHANGELOG.md`、`ROADMAP.md`、`README.md`。

指令從 repo 根目錄執行。後端測試從 `src-tauri/` 跑。每個 commit 結尾加:
```
Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 1：EFF 字典檔

**Files:**
- Create: `src-tauri/src/eff_large_wordlist.txt`

- [ ] **Step 1: 下載並去骰號**

Run:
```bash
curl -s --max-time 30 https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt \
  | awk '{print $2}' > src-tauri/src/eff_large_wordlist.txt
```
（EFF 原檔每行是 `骰號<TAB>單字`;`awk '{print $2}'` 只留單字。)

- [ ] **Step 2: 驗證**

Run:
```bash
wc -l < src-tauri/src/eff_large_wordlist.txt
head -1 src-tauri/src/eff_large_wordlist.txt
tail -1 src-tauri/src/eff_large_wordlist.txt
```
Expected:`7776`、`abacus`、`zoom`。若 `wc -l` 不是 7776,或網路失敗,停下回報(不要提交不完整的檔案)。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/eff_large_wordlist.txt
git commit -m "feat(generator): embed EFF large wordlist (CC-BY-3.0-US)"
```

---

## Task 2：generator 模組

**Files:**
- Create: `src-tauri/src/generator.rs`
- Modify: `src-tauri/src/lib.rs`(加 `pub mod generator;`)

- [ ] **Step 1: 在 lib.rs 宣告模組**

`src-tauri/src/lib.rs` 模組清單為字母序(`audit` … `vault`)。在 `pub mod error;` 之後加:
```rust
pub mod generator;
```

- [ ] **Step 2: 寫失敗測試 + 型別骨架**

建立 `src-tauri/src/generator.rs`,先放型別、`include_str!`、`todo!()` 的 `generate`,以及測試:
```rust
use rand::seq::SliceRandom;
use rand::Rng;
use serde::Deserialize;
use zeroize::Zeroize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenMode {
    Chars,
    Passphrase,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenOptions {
    pub mode: GenMode,
    pub length: usize,
    pub symbols: bool,
    pub avoid_ambiguous: bool,
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub number: bool,
}

const AMBIGUOUS: &[u8] = b"0O1lI";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{};:,.?/";

// EFF Large Wordlist (7776 words). CC-BY-3.0-US. https://www.eff.org/dice
static WORDLIST: &str = include_str!("eff_large_wordlist.txt");

pub fn generate(_opts: &GenOptions) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars_opts() -> GenOptions {
        GenOptions {
            mode: GenMode::Chars,
            length: 20,
            symbols: true,
            avoid_ambiguous: false,
            words: 5,
            separator: "-".into(),
            capitalize: true,
            number: false,
        }
    }

    fn phrase_opts() -> GenOptions {
        GenOptions { mode: GenMode::Passphrase, ..chars_opts() }
    }

    #[test]
    fn chars_respects_length_and_clamps() {
        assert_eq!(generate(&GenOptions { length: 20, ..chars_opts() }).chars().count(), 20);
        assert_eq!(generate(&GenOptions { length: 4, ..chars_opts() }).chars().count(), 8);
        assert_eq!(generate(&GenOptions { length: 999, ..chars_opts() }).chars().count(), 128);
    }

    #[test]
    fn chars_without_symbols_is_alphanumeric() {
        let pw = generate(&GenOptions { symbols: false, ..chars_opts() });
        assert!(pw.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn chars_avoid_ambiguous_excludes_confusables() {
        let pw = generate(&GenOptions { avoid_ambiguous: true, length: 128, ..chars_opts() });
        assert!(!pw.contains(['0', 'O', '1', 'l', 'I']));
    }

    #[test]
    fn passphrase_has_word_count() {
        let pw = generate(&GenOptions { words: 5, number: false, ..phrase_opts() });
        assert_eq!(pw.split('-').count(), 5);
    }

    #[test]
    fn passphrase_capitalizes_each_word() {
        let pw = generate(&GenOptions { words: 4, number: false, capitalize: true, ..phrase_opts() });
        assert!(pw.split('-').all(|w| w.chars().next().is_some_and(|c| c.is_uppercase())));
    }

    #[test]
    fn passphrase_number_appends_a_safe_digit() {
        let pw = generate(&GenOptions { number: true, ..phrase_opts() });
        let last = pw.chars().last().unwrap();
        assert!(('2'..='9').contains(&last));
    }

    #[test]
    fn wordlist_is_the_eff_large_list() {
        assert_eq!(WORDLIST.lines().count(), 7776);
        assert_eq!(WORDLIST.lines().next().unwrap(), "abacus");
        assert_eq!(WORDLIST.lines().last().unwrap(), "zoom");
    }

    #[test]
    fn two_generations_differ() {
        assert_ne!(generate(&chars_opts()), generate(&chars_opts()));
    }
}
```

- [ ] **Step 3: 跑測試確認失敗**

Run: `cd src-tauri && cargo test --lib generator::`
Expected:編譯成功但 `generate` panic(`todo!()`)→ FAIL。

- [ ] **Step 4: 實作**

把 `generate` 與 helper 換成:
```rust
pub fn generate(opts: &GenOptions) -> String {
    match opts.mode {
        GenMode::Chars => gen_chars(opts),
        GenMode::Passphrase => gen_passphrase(opts),
    }
}

fn gen_chars(opts: &GenOptions) -> String {
    let length = opts.length.clamp(8, 128);
    let mut alphabet: Vec<u8> =
        (b'a'..=b'z').chain(b'A'..=b'Z').chain(b'0'..=b'9').collect();
    if opts.symbols {
        alphabet.extend_from_slice(SYMBOLS);
    }
    if opts.avoid_ambiguous {
        alphabet.retain(|c| !AMBIGUOUS.contains(c));
    }
    let mut rng = rand::thread_rng();
    let pw: String = (0..length)
        .map(|_| *alphabet.choose(&mut rng).expect("alphabet non-empty") as char)
        .collect();
    alphabet.zeroize();
    pw
}

fn gen_passphrase(opts: &GenOptions) -> String {
    let count = opts.words.clamp(3, 12);
    let list: Vec<&str> = WORDLIST.lines().collect();
    let sep = if opts.separator.is_empty() { "-" } else { opts.separator.as_str() };
    let mut rng = rand::thread_rng();
    let mut parts: Vec<String> = (0..count)
        .map(|_| {
            let w = *list.choose(&mut rng).expect("wordlist non-empty");
            if opts.capitalize {
                capitalize(w)
            } else {
                w.to_string()
            }
        })
        .collect();
    let mut pass = parts.join(sep);
    if opts.number {
        pass.push(char::from(b'0' + rng.gen_range(2..=9)));
    }
    parts.zeroize();
    pass
}

fn capitalize(w: &str) -> String {
    let mut chars = w.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
```

- [ ] **Step 5: 跑測試確認通過**

Run: `cd src-tauri && cargo test --lib generator::`
Expected:PASS(8 個測試)。`cargo build --lib` warning-free。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/generator.rs src-tauri/src/lib.rs
git commit -m "feat(generator): chars + passphrase generation with options"
```

---

## Task 3：命令改吃 GenOptions

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: 替換命令**

把 `src-tauri/src/commands.rs` 既有的 `generate_password`(`pub fn generate_password(length: usize, include_symbols: bool) -> String { ... }`,連同其內聯的字母表/rand/zeroize 邏輯)整段換成:
```rust
#[tauri::command]
pub fn generate_password(opts: crate::generator::GenOptions) -> String {
    crate::generator::generate(&opts)
}
```
（命令名不變,`lib.rs` 的 `invoke_handler` 不需改。`#[tauri::command]` 屬性保留。）

- [ ] **Step 2: 編譯 + 測試**

Run: `cd src-tauri && cargo test --lib`
Expected:全部 PASS(既有 71 + generator 8 = 79);`cargo build --lib` warning-free(舊內聯邏輯移除後不應殘留未使用 import;若有,一併移除)。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(generator): command takes GenOptions"
```

---

## Task 4：前端型別

**Files:**
- Modify: `src/lib/types.ts`

> ipc 簽章變更刻意留到 Task 6 與 ItemDetail 一起改,讓本任務(與 Task 5)的
> 每個 commit 都保持 tsc 乾淨。

- [ ] **Step 1: 加型別**

在 `src/lib/types.ts` 檔尾加:
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

- [ ] **Step 2: 型別檢查**

Run: `pnpm tsc --noEmit`
Expected:乾淨(只是新增未使用的型別,不影響既有程式)。

- [ ] **Step 3: Commit**

```bash
git add src/lib/types.ts
git commit -m "feat(generator): frontend GenOptions type"
```

---

## Task 5：i18n 字串

**Files:**
- Modify: `src/lib/i18n.tsx`

- [ ] **Step 1: 在 `en` 字典加入**(新增 `// Generator` 小節):
```ts
  // Generator
  gen_panel_toggle: "Generator",
  gen_mode_random: "Random",
  gen_mode_passphrase: "Passphrase",
  gen_length: "Length",
  gen_symbols: "Symbols",
  gen_avoid_ambiguous: "Avoid ambiguous (0 O 1 l I)",
  gen_words: "Words",
  gen_separator: "Separator",
  gen_sep_space: "Space",
  gen_capitalize: "Capitalize",
  gen_number: "Add a number",
  gen_generate: "Generate",
```

- [ ] **Step 2: 在 `zh-TW` 字典加入對應**:
```ts
  // Generator
  gen_panel_toggle: "產生器",
  gen_mode_random: "隨機字元",
  gen_mode_passphrase: "Passphrase",
  gen_length: "長度",
  gen_symbols: "符號",
  gen_avoid_ambiguous: "排除易混淆 (0 O 1 l I)",
  gen_words: "詞數",
  gen_separator: "分隔符",
  gen_sep_space: "空格",
  gen_capitalize: "首字母大寫",
  gen_number: "加數字",
  gen_generate: "產生",
```

- [ ] **Step 3: 驗證 parity**

Run: `pnpm vitest run src/lib/i18n.test.tsx`
Expected:PASS（`Dict` 型別強制兩字典鍵一致)。

- [ ] **Step 4: Commit**

```bash
git add src/lib/i18n.tsx
git commit -m "i18n: add generator panel strings (EN + zh-TW)"
```

---

## Task 6：ipc 簽章 + ItemDetail 產生器面板

**Files:**
- Modify: `src/lib/ipc.ts`、`src/routes/ItemDetail.tsx`、`src/routes/ItemDetail.test.tsx`、`src/App.css`

> 本任務同時改 ipc 簽章與唯一呼叫者 ItemDetail,讓整體 tsc 一次到位、不留中間的破壞狀態。

- [ ] **Step 0: 改 ipc 簽章**

在 `src/lib/ipc.ts`,把既有的:
```ts
  generatePassword: (length: number, includeSymbols: boolean) =>
    invoke<string>("generate_password", { length, includeSymbols }),
```
換成(並在頂部 `import type { ... }` 加入 `GenOptions`):
```ts
  generatePassword: (opts: GenOptions) =>
    invoke<string>("generate_password", { opts }),
```

- [ ] **Step 1: 改寫產生器測試(失敗)**

在 `src/routes/ItemDetail.test.tsx`:
（a)頂部 import 改為含 `fireEvent`:
```tsx
import { screen, fireEvent } from "@testing-library/react";
```
（b)把既有的 "fills the password field from the generator" 測試整段換成這兩個:
```tsx
  it("opens the generator panel and generates with chosen options", async () => {
    ipc.generatePassword.mockResolvedValue("GENERATEDpassword20!");
    const user = userEvent.setup();
    renderDetail("new");

    await user.click(screen.getByRole("button", { name: "Generator" }));
    await user.click(screen.getByRole("button", { name: "Generate" }));

    expect(ipc.generatePassword).toHaveBeenCalledWith(
      expect.objectContaining({ mode: "chars", length: 20, symbols: true }),
    );
    expect(
      await screen.findByDisplayValue("GENERATEDpassword20!"),
    ).toBeInTheDocument();
  });

  it("generates a passphrase with the chosen word count", async () => {
    ipc.generatePassword.mockResolvedValue("Word-Word-Word-Word-Word-Word-Word");
    const user = userEvent.setup();
    renderDetail("new");

    await user.click(screen.getByRole("button", { name: "Generator" }));
    await user.click(screen.getByRole("button", { name: "Passphrase" }));
    fireEvent.change(screen.getByRole("slider"), { target: { value: "7" } });
    await user.click(screen.getByRole("button", { name: "Generate" }));

    expect(ipc.generatePassword).toHaveBeenCalledWith(
      expect.objectContaining({ mode: "passphrase", words: 7 }),
    );
  });
```

- [ ] **Step 2: 跑測試確認失敗**

Run: `pnpm vitest run src/routes/ItemDetail.test.tsx`
Expected:新兩案 FAIL(無 "Generator" 按鈕 / 面板);其餘既有案大多仍 PASS。

- [ ] **Step 3: 實作面板**

在 `src/routes/ItemDetail.tsx`:
（a)頂部 `import type { ... }` 加入 `GenOptions`。
（b)加狀態(其他 `useState` 旁):
```tsx
  const [genOpen, setGenOpen] = useState(false);
  const [gen, setGen] = useState<GenOptions>({
    mode: "chars",
    length: 20,
    symbols: true,
    avoid_ambiguous: false,
    words: 5,
    separator: "-",
    capitalize: true,
    number: true,
  });
  function setGenOpt<K extends keyof GenOptions>(k: K, v: GenOptions[K]) {
    setGen((g) => ({ ...g, [k]: v }));
  }
```
（c)把 `generate()` 換成以 `gen` 呼叫:
```tsx
  async function generate() {
    const pw = await ipc.generatePassword(gen);
    patch("password", pw);
    setConfirmWeak(false);
    setShowPw(true);
  }
```
（d)把密碼 `.pw-row` 裡原本的「產生」按鈕(`onClick={generate}`,文字 `t("detail_generate")`)改成**展開鈕**:
```tsx
            <button type="button" onClick={() => setGenOpen((o) => !o)}>
              {t("gen_panel_toggle")}
            </button>
```
（e)在密碼 `</label>` 之後(URL `<label>` 之前)插入面板:
```tsx
        {genOpen && (
          <div className="gen-panel">
            <div className="gen-seg">
              <button
                type="button"
                className={gen.mode === "chars" ? "active" : ""}
                onClick={() => setGenOpt("mode", "chars")}
              >
                {t("gen_mode_random")}
              </button>
              <button
                type="button"
                className={gen.mode === "passphrase" ? "active" : ""}
                onClick={() => setGenOpt("mode", "passphrase")}
              >
                {t("gen_mode_passphrase")}
              </button>
            </div>

            {gen.mode === "chars" ? (
              <>
                <label className="gen-row">
                  <span>{t("gen_length")}</span>
                  <input
                    type="range"
                    min={8}
                    max={64}
                    value={gen.length}
                    onChange={(e) => setGenOpt("length", Number(e.target.value))}
                  />
                  <span className="gen-val">{gen.length}</span>
                </label>
                <label className="gen-row">
                  <span>{t("gen_symbols")}</span>
                  <input
                    type="checkbox"
                    checked={gen.symbols}
                    onChange={(e) => setGenOpt("symbols", e.target.checked)}
                  />
                </label>
                <label className="gen-row">
                  <span>{t("gen_avoid_ambiguous")}</span>
                  <input
                    type="checkbox"
                    checked={gen.avoid_ambiguous}
                    onChange={(e) => setGenOpt("avoid_ambiguous", e.target.checked)}
                  />
                </label>
              </>
            ) : (
              <>
                <label className="gen-row">
                  <span>{t("gen_words")}</span>
                  <input
                    type="range"
                    min={3}
                    max={12}
                    value={gen.words}
                    onChange={(e) => setGenOpt("words", Number(e.target.value))}
                  />
                  <span className="gen-val">{gen.words}</span>
                </label>
                <label className="gen-row">
                  <span>{t("gen_separator")}</span>
                  <select
                    value={gen.separator}
                    onChange={(e) => setGenOpt("separator", e.target.value)}
                  >
                    <option value="-">-</option>
                    <option value=".">.</option>
                    <option value="_">_</option>
                    <option value=" ">{t("gen_sep_space")}</option>
                  </select>
                </label>
                <label className="gen-row">
                  <span>{t("gen_capitalize")}</span>
                  <input
                    type="checkbox"
                    checked={gen.capitalize}
                    onChange={(e) => setGenOpt("capitalize", e.target.checked)}
                  />
                </label>
                <label className="gen-row">
                  <span>{t("gen_number")}</span>
                  <input
                    type="checkbox"
                    checked={gen.number}
                    onChange={(e) => setGenOpt("number", e.target.checked)}
                  />
                </label>
              </>
            )}

            <button type="button" className="gen-go" onClick={generate}>
              {t("gen_generate")}
            </button>
          </div>
        )}
```

- [ ] **Step 4: 加樣式**

附加到 `src/App.css`:
```css
.gen-panel {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 12px;
  margin: 4px 0 12px;
}
.gen-seg {
  display: flex;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  margin-bottom: 10px;
}
.gen-seg button {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--muted);
  padding: 7px;
  cursor: pointer;
}
.gen-seg button.active {
  background: var(--accent);
  color: var(--accent-fg);
}
.gen-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin: 9px 0;
  font-size: 0.88rem;
}
.gen-row input[type="range"] {
  flex: 1;
}
.gen-val {
  min-width: 2.5ch;
  text-align: right;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}
.gen-go {
  width: 100%;
  margin-top: 10px;
}
```

- [ ] **Step 5: 跑測試 + 型別檢查**

Run: `pnpm vitest run src/routes/ItemDetail.test.tsx`
Expected:PASS(含新兩案)。
Run: `pnpm tsc --noEmit`
Expected:乾淨(ItemDetail 已改用新簽章)。
Run: `pnpm test`
Expected:整體前端套件全綠。

- [ ] **Step 6: Commit**

```bash
git add src/lib/ipc.ts src/routes/ItemDetail.tsx src/routes/ItemDetail.test.tsx src/App.css
git commit -m "feat(generator): ipc signature + inline generator panel"
```

---

## Task 7：文件 + 完整驗證

**Files:**
- Modify: `CHANGELOG.md`、`ROADMAP.md`、`README.md`

- [ ] **Step 1: 跑完整套件並記下數字**

Run: `pnpm test`
Expected:全部 PASS。記下新總數(原 78,本計畫淨變動:移除 1 個舊產生器測試、新增 2 個 → 79;以實際為準)。
Run: `cd src-tauri && cargo test --lib`
Expected:全部 PASS(原 71 + generator 8 = 79)。

- [ ] **Step 2: 更新 CHANGELOG**(英文,`## [Unreleased]` → `### Added` 最上方):
```markdown
- **Generator upgrade.** The password generator now has an inline panel on
  the item screen with a configurable length, a symbols toggle, an
  avoid-ambiguous-characters toggle, and a passphrase mode (EFF large
  wordlist) with adjustable word count, separator, capitalization, and an
  optional trailing number.
```

- [ ] **Step 3: 更新 ROADMAP（繁中)** —— 把「### 功能」的產生器升級那項換成:
```markdown
- **產生器升級。** ✅ ItemDetail 內聯產生器面板:可調長度、符號開關、排除
  易混淆字元,以及 passphrase 模式(EFF 7776 詞,可調詞數/分隔符/首字母大寫/
  加數字)。
```

- [ ] **Step 4: 更新 README 測試數字** —— Rust 測試數字行改為 Step 1 的後端數字(預期 79),前端測試數字行改為前端數字(預期 79)。

- [ ] **Step 5: 完整套件再跑一次**

Run: `pnpm test && (cd src-tauri && cargo test --lib)`
Expected:全綠,數字與 README 一致。

- [ ] **Step 6: Commit**

```bash
git add CHANGELOG.md ROADMAP.md README.md
git commit -m "docs: record generator upgrade; bump test counts"
```

---

## 自我審查筆記

- **Spec 覆蓋:** 字典(§EFF 字典)→ Task 1;generator 模組 chars/passphrase/ambiguous(§後端)→ Task 2;命令(§命令)→ Task 3;前端型別/ipc(§前端)→ Task 4;i18n → Task 5;ItemDetail 面板(§前端 ItemDetail)→ Task 6;文件 → Task 7。全部覆蓋。
- **簽章破壞性變更:** `generatePassword(length, includeSymbols)` → `generatePassword(opts)`。ipc 簽章變更與唯一呼叫者 ItemDetail、以及既有產生器測試(斷言 `(20, true)`)的改寫,全部集中在 Task 6,故每個 commit 的 tsc 都保持乾淨(Task 4 只加型別、Task 5 只加 i18n)。
- **passphrase 結尾數字:** 直接接一位 `2..=9`(不加分隔符,對應 mockup);測試 `passphrase_has_word_count` 用 `number:false` 以免數字影響 split 計數;`passphrase_number_appends_a_safe_digit` 單獨驗證末位數字。
- **型別一致:** Rust `GenOptions`(serde snake_case 欄位 `avoid_ambiguous`)對應 TS 同名;`GenMode` serde `rename_all="snake_case"` → `"chars"`/`"passphrase"` 對應 TS `type GenMode`。
- **按鈕命名歧義:** 展開鈕 `gen_panel_toggle`="Generator",面板動作 `gen_generate`="Generate";`getByRole(name:"Generate")` 為精確比對,只命中面板按鈕,不會撞到 "Generator"。
</content>
