# 密碼歷史 — 設計

日期:2026-06-07
狀態:已核准(待實作)

## 目標

讓每筆項目保留它過去用過的密碼,使用者在 ItemDetail 可檢視、複製、或一鍵
還原舊密碼。實作 `ROADMAP.md` 的「每筆項目的密碼歷史」。這是密碼衛生兩個
獨立 cycle 的第二個(第一個是重用/弱密碼 audit,已完成)。

## 做法

密碼歷史是每筆項目的資料。在後端 `VaultItem` 加一個歷史欄位;當項目的密碼
在儲存時確實變更,後端在純函式 `crud::update_item` 中把**舊密碼**記入該項目
的歷史(最新在前,上限 10 筆)。`get_item` 本就回傳完整 `VaultItem`,歷史
隨之帶到前端,由 ItemDetail 顯示。複製走既有的剪貼簿自動清除路徑;還原是
前端把舊密碼填回密碼欄。

(已否決:獨立的 `get_password_history` 命令延後載入歷史明文 —— ItemDetail
本就持有當前密碼明文,延後載入的安全收益有限,徒增一個命令與往返。)

## 資料模型(`src-tauri/src/vault/mod.rs`)

新增型別,並在 `VaultItem` 加一個 `#[serde(default)]` 欄位(舊 vault 解出為
空歷史 —— 向後相容,免遷移):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct PasswordHistoryEntry {
    pub password: String,
    #[zeroize(skip)]
    pub changed_at: i64,
}

// 在 VaultItem 內(其他欄位之後、created_at 之前的合理位置):
    #[serde(default)]
    pub password_history: Vec<PasswordHistoryEntry>,
```

- `password` 受 `ZeroizeOnDrop` 保護;`changed_at` 是 unix 秒,`#[zeroize(skip)]`。
- `VaultItem` 既有的所有建構處(`ItemInput::into_vault_item`、各測試的 `item()`
  helper)都要補上 `password_history: Vec::new()`。
- `ItemInput` **不**加此欄 —— 前端不管理歷史,後端維護。

## 擷取邏輯(`src-tauri/src/vault/crud.rs`,純函式)

新增常數 `pub const MAX_HISTORY: usize = 10;`。修改 `update_item`:
```rust
pub fn update_item(vault: &mut Vault, updated: VaultItem) -> AppResult<()> {
    let id = updated.id;
    let pos = vault
        .items
        .iter()
        .position(|i| i.id == id)
        .ok_or(AppError::ItemNotFound(id))?;

    let created_at = vault.items[pos].created_at;

    // 在取代前,若密碼確實變更且舊密碼非空,把舊密碼記入歷史(最新在前)。
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

- `updated` 來自 `into_vault_item`,其 `password_history` 為空;這裡一律以舊
  項目的歷史重建並指派,故前端送來的內容不影響歷史。
- 密碼沒變(例如只改使用者名稱)→ 不新增、歷史原樣保留。
- 舊密碼為空(項目原本沒設密碼)→ 不記空字串。
- 例:A→B→A 會得到 `[B, A]`(最新在前)。

## 命令(`src-tauri/src/commands.rs` + `lib.rs`)

- `get_item` 已回傳完整 `VaultItem` → 歷史隨之帶出,不需新命令。
- 新增 `copy_history_password`,鏡像既有的 `copy_password`(同樣的
  `ClipboardState` 自動清除路徑),但複製的是指定 index 的歷史密碼:
```rust
#[tauri::command]
pub fn copy_history_password(
    id: Uuid,
    index: usize,
    state: State<'_, AppState>,
    clipboard: State<'_, ClipboardState>,
) -> AppResult<()> {
    state.with_session(|s| {
        let item = s
            .vault
            .items
            .iter()
            .find(|i| i.id == id)
            .ok_or(AppError::ItemNotFound(id))?;
        let entry = item
            .password_history
            .get(index)
            .ok_or(AppError::ItemNotFound(id))?;
        clipboard.write_with_auto_clear(entry.password.clone(), CLIPBOARD_CLEAR)?;
        Ok(())
    })
}
```
> 註:`CLIPBOARD_CLEAR`(自動清除時長)與 `clipboard: State<ClipboardState>`
> 的取得方式,實作時對照既有 `copy_password` 照抄。在 `lib.rs` 的
> `invoke_handler` 註冊 `commands::copy_history_password`。index 越界回錯誤
> (沿用 `AppError::ItemNotFound` 即可,屬不可達的程式錯誤路徑)。

## 前端

### `src/lib/types.ts`
```ts
export interface PasswordHistoryEntry {
  password: string;
  changed_at: number;
}
// VaultItem 內加(可選,讓既有測試 mock 不需全部補欄位):
  password_history?: PasswordHistoryEntry[];
```

### `src/lib/ipc.ts`
```ts
copyHistoryPassword: (id: string, index: number) =>
  invoke<void>("copy_history_password", { id, index }),
```

### `src/routes/ItemDetail.tsx`
- 新增 state `const [history, setHistory] = useState<PasswordHistoryEntry[]>([])`,
  載入項目時 `setHistory(it.password_history ?? [])`。
- 在表單密碼欄之後、**僅當 `history.length > 0`** 顯示「密碼歷史」區:
  - 區段一個「顯示/隱藏」切換(重用 `detail_show`/`detail_hide`,預設遮罩)。
  - 每列:遮罩或明文的舊密碼、更改日期(`new Date(changed_at * 1000).toLocaleDateString()`)、
    **複製**按鈕(呼叫 `ipc.copyHistoryPassword(savedItemId, index)`,沿用既有
    「已複製」短暫提示模式)、**還原**按鈕(`patch("password", entry.password)`
    並 `setConfirmWeak(false)`,使用者之後儲存才生效)。
- 歷史僅對已儲存項目顯示(新項目沒有歷史)。`form`(= `ItemInput` 形狀)不含
  歷史;歷史是獨立的顯示資料。

### i18n（`src/lib/i18n.tsx`,EN + zh-TW)

| Key | EN | 繁中 |
|-----|----|----|
| `detail_history_section` | Password history | 密碼歷史 |
| `detail_history_copy` | Copy | 複製 |
| `detail_history_copied` | Copied | 已複製 |
| `detail_history_restore` | Use again | 重新使用 |

區段的顯示/隱藏切換沿用既有的 `detail_show` / `detail_hide`(通用「顯示/
隱藏」),不另立新鍵。

### 樣式（`src/App.css`)
附加一個 `history-*` 小節(歷史列、遮罩文字、日期、按鈕),沿用 palette 變數。

## 測試

**後端 — `src-tauri/src/vault/crud.rs`(`#[cfg(test)]`):**
- 變更密碼後,舊密碼被記入歷史(len 1、值正確、`changed_at > 0`)。
- 未變更密碼(只改其他欄位)→ 歷史不變。
- A→B→A → 歷史為 `[B, A]`(最新在前)。
- 連續 11 次變更 → 歷史上限 10、最舊被丟棄。
- 舊密碼為空 → 不記空字串。
- serde round-trip:無 `password_history` 欄位的舊 JSON 解出為空歷史(向後相容)。

**前端 — `src/routes/ItemDetail.test.tsx`(新增):**
- `getItem` 回傳含 `password_history` 的項目 → 顯示「密碼歷史」區與日期。
- 點「顯示」後可見舊密碼明文。
- 點「還原」把舊密碼填回密碼欄(以 `getByDisplayValue` 或欄位值驗證)。
- 點「複製」呼叫 `ipc.copyHistoryPassword` 並帶正確 (id, index)。
- 無 `password_history`(或空)→ 不顯示該區。

## 不做(YAGNI)

- 單筆刪除歷史 / 清空整個歷史。
- TOTP 密鑰歷史。
- 舊密碼的差異(diff)檢視。
- 匯出/備份的特殊處理(歷史是 vault 欄位,既有加密備份/還原自動帶上;舊備份
  無此欄 → 解出為空歷史)。
- 每列各自獨立的 reveal(採區段單一切換)。

## 完成時要更新的文件

- `CHANGELOG.md`(Unreleased → Added,英文)。
- `ROADMAP.md`(將「密碼歷史」標為完成)。
- `README.md`(若測試數字那行需更新)。
</content>
