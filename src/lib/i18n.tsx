import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { ipc } from "./ipc";

/** Languages the UI ships translations for. */
export type Lang = "en" | "zh-TW";
/** What the user can pick: a concrete language, or "follow the OS". */
export type LocalePref = "system" | Lang;

/**
 * Translation dictionaries. `en` is the reference set — every key here must
 * exist in `zh-TW` too (enforced by the `Dict` type). Values may contain
 * `{name}`-style placeholders, filled by the `vars` argument to `t`.
 */
const en = {
  // generic
  loading: "Loading…",
  cancel: "Cancel",
  save: "Save",
  back: "← Back",
  delete: "Delete",

  // Setup
  setup_title: "Welcome to Keytainer",
  setup_intro_1: "Set a master password to encrypt your local vault.",
  setup_intro_2: "If you lose the master password it cannot be recovered — choose one you can remember and that's strong enough.",
  setup_pw_label: "Master password (at least 8 characters)",
  setup_confirm_label: "Type it again",
  setup_mismatch: "The two entries don't match",
  setup_creating: "Creating…",
  setup_create_btn: "Create vault",

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

  // Unlock
  unlock_title: "Unlock Keytainer",
  unlock_idle: "Locked automatically (idle too long)",
  unlock_keychain_btn: "🔑 One-tap unlock with system keychain",
  unlock_pw_label: "Master password",
  unlock_wrong_pw: "Wrong master password",
  unlock_corrupt: "Vault file is corrupt or not a Keytainer file",
  unlock_keychain_unavailable: "Keychain unavailable, use your master password instead",
  unlock_keychain_mismatch: "The keychain key doesn't match the vault, use your master password",
  unlock_unlocking: "Unlocking…",
  unlock_btn: "Unlock",

  // List
  list_add: "＋ Add",
  list_lock: "🔒 Lock",
  list_search: "Search site, account, tag…",
  list_all: "All",
  list_no_match: "No matching items",
  list_empty: "No items yet — tap \"Add\" at the top right to start",
  list_unnamed: "(unnamed)",

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

  // ItemDetail
  detail_new: "New item",
  detail_copy_pw: "Copy password",
  detail_copied_pw: "Password copied ✓",
  detail_site: "Site name",
  detail_username: "Account",
  detail_password: "Password",
  detail_show: "Show",
  detail_hide: "Hide",
  detail_generate: "Generate",
  detail_url: "URL (optional)",
  detail_2fa: "2FA",
  detail_add_2fa: "Add 2FA (TOTP secret)",
  detail_tags: "Tags (comma separated)",
  detail_tags_ph: "e.g. work, finance",
  detail_notes: "Notes",
  detail_saving: "Saving…",
  detail_confirm_delete: "Delete this item?",

  // Password history
  detail_history_section: "Password history",
  detail_history_show: "Show passwords",
  detail_history_hide: "Hide passwords",
  detail_history_copy: "Copy",
  detail_history_copied: "Copied",
  detail_history_restore: "Use again",

  totp_secret: "Secret (base32)",
  totp_algorithm: "Algorithm",
  totp_digits: "Digits",
  totp_period: "Period (seconds)",
  totp_remove: "Remove 2FA",

  // Settings
  settings_title: "Settings",
  settings_sec_security: "Security",
  settings_autolock: "Auto-lock (seconds) — re-ask for the master password after this idle time",
  settings_clipboard: "Clipboard auto-clear (seconds) — how long after copying a password/2FA to wipe it",
  settings_show_totp: "Show the current 2FA code (uncheck to mask with ●●●●●●)",
  settings_saving: "Saving…",
  settings_saved: "Saved ✓",
  settings_save_btn: "Save settings",
  settings_sec_language: "Language",
  settings_language_label: "Interface language",
  settings_lang_system: "Follow system",
  settings_sec_password: "Master password",
  settings_change_pw: "Change master password",
  settings_current_pw: "Current password",
  settings_new_pw: "New password (at least 8 characters)",
  settings_new_pw_confirm: "Confirm new password",
  settings_change_pw_btn: "Change password",
  settings_pw_changed: "Master password changed ✓",
  settings_pw_wrong_current: "Current password is wrong",
  settings_sec_quickunlock: "Quick unlock",
  settings_keychain_unavailable: "This machine can't use the system keychain (on Linux you need Secret Service / GNOME Keyring running).",
  settings_keychain_toggle: "Store the current decryption key in the system keychain for one-tap unlock next launch (no master password needed)",
  settings_keychain_warning: "⚠ This stores the raw vault key in your OS keychain, so vault security then depends on your OS account — anyone who can read your keychain can open the vault without the master password.",
  settings_sec_backup: "Backup / Restore",
  settings_backup_intro: "Export encrypts the whole vault with the \"backup password\" you enter (can differ from the master password). Import merges: same id is overwritten, new id is added.",
  settings_backup_pw: "Backup password",
  settings_backup_pw_ph: "(needed for both export and import)",
  settings_export: "Export…",
  settings_import: "Import…",
  settings_export_done: "Exported encrypted backup to {path}",
  settings_import_done: "Imported: {added} added, {updated} updated",
  settings_backup_wrong_pw: "Wrong backup password",
  settings_backup_corrupt: "File is corrupt or not a Keytainer backup",
  settings_export_dialog: "Export Keytainer backup",
  settings_import_dialog: "Choose a Keytainer backup file",
  settings_sec_updates: "Updates",
  settings_check_updates: "Check for updates",
  settings_checking: "Checking…",
  settings_up_to_date: "You're on the latest version ✓",
  settings_update_available: "Update available: {version}. Download and install?",
  settings_update_install: "Install update",
  settings_update_installing: "Downloading and installing…",
  settings_update_done: "Updated — the app will restart",
  settings_update_failed: "Update check failed: {message}",
} as const;

type Dict = Record<keyof typeof en, string>;

const zhTW: Dict = {
  loading: "載入中…",
  cancel: "取消",
  save: "儲存",
  back: "← 返回",
  delete: "刪除",

  setup_title: "歡迎使用 Keytainer",
  setup_intro_1: "設定一個主密碼來加密你的本機保險庫。",
  setup_intro_2: "主密碼遺失就無法復原，請選擇你能記住、且夠強的密碼。",
  setup_pw_label: "主密碼（至少 8 字元）",
  setup_confirm_label: "再輸入一次",
  setup_mismatch: "兩次輸入不一致",
  setup_creating: "建立中…",
  setup_create_btn: "建立保險庫",

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

  unlock_title: "解鎖 Keytainer",
  unlock_idle: "已自動鎖定（閒置過久）",
  unlock_keychain_btn: "🔑 用系統 keychain 一鍵解鎖",
  unlock_pw_label: "主密碼",
  unlock_wrong_pw: "主密碼錯誤",
  unlock_corrupt: "保險庫檔案損毀或非 Keytainer 檔案",
  unlock_keychain_unavailable: "Keychain 無法存取，請改用主密碼",
  unlock_keychain_mismatch: "Keychain 內的金鑰跟保險庫對不起來，請改用主密碼",
  unlock_unlocking: "解鎖中…",
  unlock_btn: "解鎖",

  list_add: "＋ 新增",
  list_lock: "🔒 鎖定",
  list_search: "搜尋網站、帳號、標籤…",
  list_all: "全部",
  list_no_match: "沒有符合的項目",
  list_empty: "還沒有任何項目，點右上「新增」開始",
  list_unnamed: "(未命名)",

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

  detail_new: "新增項目",
  detail_copy_pw: "複製密碼",
  detail_copied_pw: "已複製密碼 ✓",
  detail_site: "網站名稱",
  detail_username: "帳號",
  detail_password: "密碼",
  detail_show: "顯示",
  detail_hide: "隱藏",
  detail_generate: "產生",
  detail_url: "網址（選填）",
  detail_2fa: "2FA",
  detail_add_2fa: "加入 2FA（TOTP secret）",
  detail_tags: "標籤（用逗號分隔）",
  detail_tags_ph: "例如：工作, 財務",
  detail_notes: "備註",
  detail_saving: "儲存中…",
  detail_confirm_delete: "確定要刪除這個項目嗎？",

  // Password history
  detail_history_section: "密碼歷史",
  detail_history_show: "顯示密碼",
  detail_history_hide: "隱藏密碼",
  detail_history_copy: "複製",
  detail_history_copied: "已複製",
  detail_history_restore: "重新使用",

  totp_secret: "Secret（base32）",
  totp_algorithm: "演算法",
  totp_digits: "位數",
  totp_period: "週期（秒）",
  totp_remove: "移除 2FA",

  settings_title: "設定",
  settings_sec_security: "安全",
  settings_autolock: "自動鎖定（秒）—— 閒置這麼久之後要求重輸主密碼",
  settings_clipboard: "剪貼簿自動清除（秒）—— 複製密碼/2FA 後多久清空",
  settings_show_totp: "顯示 2FA 當下碼（取消勾選會用 ●●●●●● 遮蔽）",
  settings_saving: "儲存中…",
  settings_saved: "已儲存 ✓",
  settings_save_btn: "儲存設定",
  settings_sec_language: "語言",
  settings_language_label: "介面語言",
  settings_lang_system: "跟隨系統",
  settings_sec_password: "主密碼",
  settings_change_pw: "變更主密碼",
  settings_current_pw: "目前密碼",
  settings_new_pw: "新密碼（至少 8 字元）",
  settings_new_pw_confirm: "確認新密碼",
  settings_change_pw_btn: "變更密碼",
  settings_pw_changed: "主密碼已變更 ✓",
  settings_pw_wrong_current: "目前密碼錯誤",
  settings_sec_quickunlock: "快速解鎖",
  settings_keychain_unavailable: "這台機器無法使用系統 keychain（Linux 上需啟動 Secret Service / GNOME Keyring 等服務）。",
  settings_keychain_toggle: "把目前的解密金鑰存進系統 keychain，下次啟動可一鍵解鎖（不再需要輸主密碼）",
  settings_keychain_warning: "⚠ 這會把原始保險庫金鑰存進作業系統 keychain，保險庫安全將取決於你的 OS 帳號 —— 任何能讀取你 keychain 的人都能不用主密碼開啟保險庫。",
  settings_sec_backup: "備份 / 還原",
  settings_backup_intro: "匯出會用你輸入的「備份密碼」加密整個保險庫（可以跟主密碼不一樣）。匯入是合併：相同 id 會被覆寫，新 id 會被加入。",
  settings_backup_pw: "備份密碼",
  settings_backup_pw_ph: "（匯出/匯入都需要）",
  settings_export: "匯出…",
  settings_import: "匯入…",
  settings_export_done: "已匯出加密備份到 {path}",
  settings_import_done: "已匯入：新增 {added} 筆、更新 {updated} 筆",
  settings_backup_wrong_pw: "備份密碼錯誤",
  settings_backup_corrupt: "檔案損毀或不是 Keytainer 備份",
  settings_export_dialog: "匯出 Keytainer 備份",
  settings_import_dialog: "選擇 Keytainer 備份檔",
  settings_sec_updates: "更新",
  settings_check_updates: "檢查更新",
  settings_checking: "檢查中…",
  settings_up_to_date: "已是最新版本 ✓",
  settings_update_available: "有可用更新：{version}。要下載並安裝嗎？",
  settings_update_install: "安裝更新",
  settings_update_installing: "下載並安裝中…",
  settings_update_done: "已更新 —— 應用程式將重新啟動",
  settings_update_failed: "檢查更新失敗：{message}",
};

const dicts: Record<Lang, Dict> = { en, "zh-TW": zhTW };

export type TKey = keyof typeof en;

/** Map an OS locale string (e.g. "zh-TW", "zh_Hant", "en-US") to a Lang. */
export function resolveSystemLang(osLocale: string): Lang {
  return osLocale.toLowerCase().startsWith("zh") ? "zh-TW" : "en";
}

/** Resolve the user preference + OS locale to a concrete UI language. */
export function resolveLang(pref: LocalePref, osLocale: string): Lang {
  return pref === "system" ? resolveSystemLang(osLocale) : pref;
}

interface I18nValue {
  lang: Lang;
  /** Translate a key, optionally interpolating `{name}` placeholders. */
  t: (key: TKey, vars?: Record<string, string | number>) => string;
  /** Apply a new preference immediately (e.g. after the user picks one). */
  setPref: (pref: LocalePref) => void;
}

const I18nContext = createContext<I18nValue | null>(null);

function translate(lang: Lang, key: TKey, vars?: Record<string, string | number>): string {
  let s: string = dicts[lang][key] ?? en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
    }
  }
  return s;
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [osLocale, setOsLocale] = useState("en");
  const [pref, setPrefState] = useState<LocalePref>("system");

  useEffect(() => {
    // Pull both the OS locale and the saved preference once on boot.
    void ipc.getSystemLocale().then(setOsLocale).catch(() => {});
    void ipc
      .getSettings()
      .then((s) => setPrefState((s.locale as LocalePref) ?? "system"))
      .catch(() => {});
  }, []);

  const lang = resolveLang(pref, osLocale);
  const value: I18nValue = {
    lang,
    t: (key, vars) => translate(lang, key, vars),
    setPref: setPrefState,
  };

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within I18nProvider");
  return ctx;
}

/** Convenience: just the translate function. */
export function useT() {
  return useI18n().t;
}
