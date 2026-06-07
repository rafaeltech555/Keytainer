# 開發藍圖（Roadmap）

Keytainer 的開發現況與接下來可能的方向。這是一份**活文件** —— 記錄專案
實際的進度,而非承諾。每個版本的精確紀錄請見 [CHANGELOG.md](CHANGELOG.md)。

目前版本:**v0.3.0**(最新)。

## 已完成（Shipped）

原本的 Phase 1–5 規劃已全部完成,並額外做了一輪 v0.2.0 的安全強化。

### Phase 1 — 金庫核心 ✅ (v0.1.0)
- 主密碼金庫:Argon2id（m=64 MiB、t=3、p=1)金鑰衍生。
- 單檔加密儲存,採 atomic-rename 寫入(防當機損毀)。

### Phase 2 — UI MVP ✅ (v0.1.0)
- Setup / Unlock / List / ItemDetail 畫面。

### Phase 3 — 即時密鑰與衛生 ✅ (v0.1.0)
- TOTP 產碼（RFC 6238、SHA-1/256/512)含即時倒數。
- 一鍵複製 + 剪貼簿自動清除（以 generation 計數判定)。
- 閒置自動鎖,並對 UI 發出 `vault-locked` 事件。

### Phase 4 — 強化功能 ✅ (v0.1.0)
- 設定畫面。
- 加密 JSON 備份/還原（非破壞式合併匯入)。
- 搜尋 + 標籤晶片篩選。
- 可選的 OS keychain 快速解鎖（Secret Service / Keychain /
  Credential Manager)。
- 密碼產生器（長度 + 符號)。

### Phase 5 — 打包與發布 ✅ (v0.1.0 → v0.1.2)
- 透過 GitHub Actions 產出跨平台安裝檔:Linux `.deb`/`.rpm`/
  `.AppImage`、macOS `.dmg`（Apple Silicon + Intel)、Windows
  `-setup.exe`/`.msi`。
- Tauri 更新器簽章金鑰接進 CI;每個 bundle 附帶獨立的 `.sig`,
  發布時一併產生 `latest.json`。

### v0.2.0 — 安全強化與打磨 ✅
- **金庫格式 v2:** 以 XChaCha20-Poly1305（192-bit nonce)取代
  AES-256-GCM,消除固定金鑰下 nonce 重用的實際界限。檔頭
  (格式版本、Argon2 參數、salt、nonce)綁定為 AEAD associated data。
  備份封套同樣強化（v2)。
- **透明遷移:** v1 金庫與 `-v1` 備份仍可開啟,下次儲存時自動升級為
  v2 —— 不掉資料、不需使用者操作。
- **應用內變更主密碼**(重新衍生、重新加密、刷新 keychain)。
- **可運作的應用內更新器:** 安裝前以內嵌金鑰驗證已簽章的
  `latest.json`。
- **English / 繁體中文** 語言切換(跟隨 OS locale,並持久化)。
- 嚴格 CSP;明確的 keychain 快速解鎖警告;閒置自動鎖不再於編輯中觸發;
  TOTP 短 HMAC 防護;產生的密碼緩衝區清除。

## 接下來（候選工作,未排程）

相對於主流密碼管理器的缺口,大致依優先順序排列。以下皆尚未納入任何版本。

### 品質與信心
- **前端測試。** ✅ Vitest + Testing Library 測試框架現已涵蓋 i18n
  resolver、TOTP 倒數（`TotpDisplay`)、鎖定導航（`App`),以及每個路由
  —— `Setup`、`Unlock`、`List`、`ItemDetail`、`Settings`(錯誤對應、
  變更密碼、locale 切換、keychain 開關、更新器、備份/還原),再加上
  密碼強度計、密碼 audit(安全檢查畫面)與密碼歷史 —— 共 78 個測試、11 個
  檔案（`pnpm test`)。
- **後端測試。** ✅ `session`(上鎖/解鎖、閒置計時器重置、失敗命令不
  算活動)、`clipboard` 自動清除的 generation/staleness、`keychain` 金鑰
  encode/decode + 32-byte malformed-key 防護,以及 `audit`(重用/弱密碼)
  與密碼歷史擷取皆已覆蓋 —— 共 71 個 Rust 測試（`cd src-tauri && cargo
  test`)。唯一尚未測試的是 `spawn_idle_watcher`,它需要 Tauri `AppHandle`,
  較適合整合測試。

### 功能
- **密碼強度計。** ✅ 在 setup、變更密碼、項目表單以 zxcvbn-ts 顯示強度;
  主密碼以 score ≥ 2 設門檻,弱的項目密碼則採軟確認儲存。
- **重複/重用密碼偵測。** ✅ 從清單進入的「安全檢查」畫面,標示重用密碼
  (≥2 項目共用)與弱密碼(zxcvbn score < 2);完全在 Rust 後端計算,密碼
  不離開後端。每個發現都連到對應項目以便修正。
- **密碼歷史。** ✅ 每筆項目保留最近 10 個用過的密碼(密碼變更時自動記錄);
  在項目畫面可檢視、複製(剪貼簿自動清除)或還原。歷史隨加密 vault 與其備份
  一併保存。
- **產生器升級。** ✅ ItemDetail 內聯產生器面板:可調長度、符號開關、排除
  易混淆字元,以及 passphrase 模式(EFF 7776 詞,可調詞數/分隔符/首字母大寫/
  加數字)。
- **瀏覽器自動填入 / 擴充套件** —— 相對主流管理器最關鍵的缺口
  (工程量大;近期不在範圍內)。

### 發布
- **OS 程式碼簽章:** macOS（Apple Developer ID)與 Windows
  (Authenticode),以去除首次啟動的 Gatekeeper / SmartScreen 警告。
  需付費憑證;既有的更新器簽章是**不同**機制(它驗證的是更新內容,
  而非 OS 層級的安裝檔)。啟用步驟(要買哪些憑證、加哪些 CI secrets、
  如何接上 tauri-action / `tauri.conf.json`)已寫在
  [README 的 OS code signing 一節](README.md#os-code-signing-not-yet-enabled)
  —— 待取得憑證後即可接線。
- **發布流程修正。** ✅ 改成三段式(`create-release` 建單一 draft →
  `build-tauri` matrix 以同一 release id 上傳並合併 `latest.json` →
  `publish-release` 取消 draft),消除原本 matrix 競態產生兩個 draft、
  分裂 `latest.json` 的問題。

## 非目標（Non-goals）

依設計,Keytainer **不**打算加入:
- 雲端同步或託管帳號（本機優先是核心)。
- 行動裝置版本。
- 防護已以本機使用者身分執行的惡意程式(見 [README](README.md#threat-model)
  的威脅模型)。
</content>
