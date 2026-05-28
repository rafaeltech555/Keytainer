import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { ipc } from "../lib/ipc";
import type { Settings as SettingsType } from "../lib/types";
import { isAppError } from "../lib/types";

interface Props {
  onClose: () => void;
}

export function Settings({ onClose }: Props) {
  const [settings, setSettings] = useState<SettingsType | null>(null);
  const [saving, setSaving] = useState(false);
  const [savedFlash, setSavedFlash] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [keychainSupported, setKeychainSupported] = useState(false);
  const [keychainEnabled, setKeychainEnabled] = useState(false);

  // Backup/restore inline state
  const [backupPw, setBackupPw] = useState("");
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([
      ipc.getSettings(),
      ipc.keychainAvailable(),
      ipc.keychainIsEnabled(),
    ])
      .then(([s, supp, en]) => {
        setSettings(s);
        setKeychainSupported(supp);
        setKeychainEnabled(en);
      })
      .catch((e) => setError(isAppError(e) ? e.message : String(e)));
  }, []);

  function patch<K extends keyof SettingsType>(key: K, value: SettingsType[K]) {
    setSettings((s) => (s ? { ...s, [key]: value } : s));
  }

  async function saveAll() {
    if (!settings || saving) return;
    setSaving(true);
    setError(null);
    try {
      await ipc.saveSettings(settings);
      setSavedFlash(true);
      setTimeout(() => setSavedFlash(false), 1500);
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  }

  async function toggleKeychain(enable: boolean) {
    setError(null);
    try {
      if (enable) {
        await ipc.keychainEnable();
      } else {
        await ipc.keychainDisable();
      }
      setKeychainEnabled(enable);
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    }
  }

  async function doExport() {
    if (!backupPw || busy) return;
    setError(null);
    setReport(null);
    const path = await save({
      title: "匯出 Keytainer 備份",
      defaultPath: `keytainer-backup-${new Date().toISOString().slice(0, 10)}.json`,
      filters: [{ name: "Keytainer backup", extensions: ["json"] }],
    });
    if (!path) return;
    setBusy(true);
    try {
      await ipc.exportVault(path, backupPw);
      setReport(`已匯出加密備份到 ${path}`);
      setBackupPw("");
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function doImport() {
    if (!backupPw || busy) return;
    setError(null);
    setReport(null);
    const picked = await open({
      title: "選擇 Keytainer 備份檔",
      multiple: false,
      filters: [{ name: "Keytainer backup", extensions: ["json"] }],
    });
    if (!picked || typeof picked !== "string") return;
    setBusy(true);
    try {
      const result = await ipc.importVault(picked, backupPw);
      setReport(`已匯入：新增 ${result.added} 筆、更新 ${result.updated} 筆`);
      setBackupPw("");
    } catch (e) {
      if (isAppError(e) && e.kind === "WrongPassword") {
        setError("備份密碼錯誤");
      } else if (isAppError(e) && e.kind === "VaultCorrupt") {
        setError("檔案損毀或不是 Keytainer 備份");
      } else {
        setError(isAppError(e) ? e.message : String(e));
      }
    } finally {
      setBusy(false);
    }
  }

  if (!settings) {
    return (
      <div className="screen centered">
        <p>{error ?? "載入中…"}</p>
      </div>
    );
  }

  return (
    <div className="screen settings-screen">
      <header className="detail-header">
        <button className="secondary" onClick={onClose}>← 返回</button>
        <h2>設定</h2>
        <span />
      </header>

      <section className="settings-section">
        <h3>安全</h3>
        <label>
          自動鎖定（秒）—— 闒置這麼久之後要求重輸主密碼
          <input
            type="number"
            min={30}
            max={86400}
            value={settings.auto_lock_seconds}
            onChange={(e) => patch("auto_lock_seconds", Number(e.target.value))}
          />
        </label>
        <label>
          剪貼簿自動清除（秒）—— 複製密碼/2FA 後多久清空
          <input
            type="number"
            min={5}
            max={300}
            value={settings.clipboard_clear_seconds}
            onChange={(e) =>
              patch("clipboard_clear_seconds", Number(e.target.value))
            }
          />
        </label>
        <label className="row-toggle">
          <input
            type="checkbox"
            checked={settings.show_totp_code}
            onChange={(e) => patch("show_totp_code", e.target.checked)}
          />
          顯示 2FA 當下碼（取消勾選會用 ●●●●●● 遮蔽）
        </label>
        <button onClick={saveAll} disabled={saving}>
          {saving ? "儲存中…" : savedFlash ? "已儲存 ✓" : "儲存設定"}
        </button>
      </section>

      <section className="settings-section">
        <h3>快速解鎖</h3>
        {!keychainSupported ? (
          <p className="muted">
            這台機器無法使用系統 keychain（Linux 上需啟動 Secret Service /
            GNOME Keyring 等服務）。
          </p>
        ) : (
          <label className="row-toggle">
            <input
              type="checkbox"
              checked={keychainEnabled}
              onChange={(e) => toggleKeychain(e.target.checked)}
            />
            把目前的解密金鑰存進系統 keychain，下次啟動可一鍵解鎖（不再需要輸主密碼）
          </label>
        )}
      </section>

      <section className="settings-section">
        <h3>備份 / 還原</h3>
        <p className="muted">
          匯出會用你輸入的「備份密碼」加密整個保險庫（可以跟主密碼不一樣）。
          匯入是合併：相同 id 會被覆寫，新 id 會被加入。
        </p>
        <label>
          備份密碼
          <input
            type="password"
            value={backupPw}
            onChange={(e) => setBackupPw(e.target.value)}
            placeholder="（匯出/匯入都需要）"
            autoComplete="off"
          />
        </label>
        <div className="form-actions">
          <button onClick={doExport} disabled={!backupPw || busy}>
            匯出…
          </button>
          <button className="secondary" onClick={doImport} disabled={!backupPw || busy}>
            匯入…
          </button>
        </div>
        {report && <div className="muted">{report}</div>}
      </section>

      {error && <div className="error">{error}</div>}
    </div>
  );
}
