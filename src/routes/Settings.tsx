import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ipc } from "../lib/ipc";
import type { Settings as SettingsType } from "../lib/types";
import { isAppError } from "../lib/types";
import { useI18n, type LocalePref } from "../lib/i18n";

interface Props {
  onClose: () => void;
}

export function Settings({ onClose }: Props) {
  const { t, setPref } = useI18n();
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

  // Change-password state
  const [curPw, setCurPw] = useState("");
  const [newPw, setNewPw] = useState("");
  const [newPw2, setNewPw2] = useState("");
  const [pwBusy, setPwBusy] = useState(false);
  const [pwMsg, setPwMsg] = useState<string | null>(null);
  const [pwErr, setPwErr] = useState<string | null>(null);

  // Updater state
  const [updMsg, setUpdMsg] = useState<string | null>(null);
  const [updBusy, setUpdBusy] = useState(false);
  const [pendingUpdate, setPendingUpdate] = useState<Awaited<ReturnType<typeof check>>>(null);

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

  async function onLocaleChange(locale: LocalePref) {
    patch("locale", locale);
    setPref(locale); // apply to the UI immediately
    // Persist right away so the choice survives a restart, merging with the
    // latest settings in state.
    if (settings) {
      try {
        await ipc.saveSettings({ ...settings, locale });
      } catch (e) {
        setError(isAppError(e) ? e.message : String(e));
      }
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

  async function changePassword() {
    setPwErr(null);
    setPwMsg(null);
    if (newPw.length < 8 || newPw !== newPw2 || pwBusy) return;
    setPwBusy(true);
    try {
      await ipc.changePassword(curPw, newPw);
      setPwMsg(t("settings_pw_changed"));
      setCurPw("");
      setNewPw("");
      setNewPw2("");
    } catch (e) {
      if (isAppError(e) && e.kind === "WrongPassword") {
        setPwErr(t("settings_pw_wrong_current"));
      } else {
        setPwErr(isAppError(e) ? e.message : String(e));
      }
    } finally {
      setPwBusy(false);
    }
  }

  async function checkForUpdates() {
    setUpdMsg(null);
    setUpdBusy(true);
    try {
      const update = await check();
      if (update) {
        setPendingUpdate(update);
        setUpdMsg(t("settings_update_available", { version: update.version }));
      } else {
        setPendingUpdate(null);
        setUpdMsg(t("settings_up_to_date"));
      }
    } catch (e) {
      setUpdMsg(t("settings_update_failed", { message: isAppError(e) ? e.message : String(e) }));
    } finally {
      setUpdBusy(false);
    }
  }

  async function installUpdate() {
    if (!pendingUpdate) return;
    setUpdBusy(true);
    setUpdMsg(t("settings_update_installing"));
    try {
      await pendingUpdate.downloadAndInstall();
      setUpdMsg(t("settings_update_done"));
      await relaunch();
    } catch (e) {
      setUpdMsg(t("settings_update_failed", { message: isAppError(e) ? e.message : String(e) }));
    } finally {
      setUpdBusy(false);
    }
  }

  async function doExport() {
    if (!backupPw || busy) return;
    setError(null);
    setReport(null);
    const path = await save({
      title: t("settings_export_dialog"),
      defaultPath: `keytainer-backup-${new Date().toISOString().slice(0, 10)}.json`,
      filters: [{ name: "Keytainer backup", extensions: ["json"] }],
    });
    if (!path) return;
    setBusy(true);
    try {
      await ipc.exportVault(path, backupPw);
      setReport(t("settings_export_done", { path }));
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
      title: t("settings_import_dialog"),
      multiple: false,
      filters: [{ name: "Keytainer backup", extensions: ["json"] }],
    });
    if (!picked || typeof picked !== "string") return;
    setBusy(true);
    try {
      const result = await ipc.importVault(picked, backupPw);
      setReport(t("settings_import_done", { added: result.added, updated: result.updated }));
      setBackupPw("");
    } catch (e) {
      if (isAppError(e) && e.kind === "WrongPassword") {
        setError(t("settings_backup_wrong_pw"));
      } else if (isAppError(e) && e.kind === "VaultCorrupt") {
        setError(t("settings_backup_corrupt"));
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
        <p>{error ?? t("loading")}</p>
      </div>
    );
  }

  const newPwOk = newPw.length >= 8 && newPw === newPw2;

  return (
    <div className="screen settings-screen">
      <header className="detail-header">
        <button className="secondary" onClick={onClose}>{t("back")}</button>
        <h2>{t("settings_title")}</h2>
        <span />
      </header>

      <section className="settings-section">
        <h3>{t("settings_sec_language")}</h3>
        <label>
          {t("settings_language_label")}
          <select
            value={settings.locale}
            onChange={(e) => onLocaleChange(e.target.value as LocalePref)}
          >
            <option value="system">{t("settings_lang_system")}</option>
            <option value="en">English</option>
            <option value="zh-TW">繁體中文</option>
          </select>
        </label>
      </section>

      <section className="settings-section">
        <h3>{t("settings_sec_security")}</h3>
        <label>
          {t("settings_autolock")}
          <input
            type="number"
            min={30}
            max={86400}
            value={settings.auto_lock_seconds}
            onChange={(e) => patch("auto_lock_seconds", Number(e.target.value))}
          />
        </label>
        <label>
          {t("settings_clipboard")}
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
          {t("settings_show_totp")}
        </label>
        <button onClick={saveAll} disabled={saving}>
          {saving ? t("settings_saving") : savedFlash ? t("settings_saved") : t("settings_save_btn")}
        </button>
      </section>

      <section className="settings-section">
        <h3>{t("settings_sec_password")}</h3>
        <label>
          {t("settings_current_pw")}
          <input
            type="password"
            value={curPw}
            onChange={(e) => setCurPw(e.target.value)}
            autoComplete="current-password"
          />
        </label>
        <label>
          {t("settings_new_pw")}
          <input
            type="password"
            value={newPw}
            onChange={(e) => setNewPw(e.target.value)}
            autoComplete="new-password"
          />
        </label>
        <label>
          {t("settings_new_pw_confirm")}
          <input
            type="password"
            value={newPw2}
            onChange={(e) => setNewPw2(e.target.value)}
            autoComplete="new-password"
          />
          {newPw2.length > 0 && newPw !== newPw2 && (
            <span className="error-inline">{t("setup_mismatch")}</span>
          )}
        </label>
        <button onClick={changePassword} disabled={!newPwOk || !curPw || pwBusy}>
          {pwBusy ? t("settings_saving") : t("settings_change_pw_btn")}
        </button>
        {pwMsg && <div className="muted">{pwMsg}</div>}
        {pwErr && <div className="error">{pwErr}</div>}
      </section>

      <section className="settings-section">
        <h3>{t("settings_sec_quickunlock")}</h3>
        {!keychainSupported ? (
          <p className="muted">{t("settings_keychain_unavailable")}</p>
        ) : (
          <>
            <label className="row-toggle">
              <input
                type="checkbox"
                checked={keychainEnabled}
                onChange={(e) => toggleKeychain(e.target.checked)}
              />
              {t("settings_keychain_toggle")}
            </label>
            <p className="muted">{t("settings_keychain_warning")}</p>
          </>
        )}
      </section>

      <section className="settings-section">
        <h3>{t("settings_sec_backup")}</h3>
        <p className="muted">{t("settings_backup_intro")}</p>
        <label>
          {t("settings_backup_pw")}
          <input
            type="password"
            value={backupPw}
            onChange={(e) => setBackupPw(e.target.value)}
            placeholder={t("settings_backup_pw_ph")}
            autoComplete="off"
          />
        </label>
        <div className="form-actions">
          <button onClick={doExport} disabled={!backupPw || busy}>
            {t("settings_export")}
          </button>
          <button className="secondary" onClick={doImport} disabled={!backupPw || busy}>
            {t("settings_import")}
          </button>
        </div>
        {report && <div className="muted">{report}</div>}
      </section>

      <section className="settings-section">
        <h3>{t("settings_sec_updates")}</h3>
        <div className="form-actions">
          <button onClick={checkForUpdates} disabled={updBusy}>
            {updBusy ? t("settings_checking") : t("settings_check_updates")}
          </button>
          {pendingUpdate && (
            <button className="secondary" onClick={installUpdate} disabled={updBusy}>
              {t("settings_update_install")}
            </button>
          )}
        </div>
        {updMsg && <div className="muted">{updMsg}</div>}
      </section>

      {error && <div className="error">{error}</div>}
    </div>
  );
}
