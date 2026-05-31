import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";
import { isAppError } from "../lib/types";
import { useT } from "../lib/i18n";

interface Props {
  onUnlocked: () => void;
  reason?: "idle" | "manual";
}

export function Unlock({ onUnlocked, reason }: Props) {
  const t = useT();
  const [pw, setPw] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [keychainOption, setKeychainOption] = useState(false);

  useEffect(() => {
    Promise.all([ipc.keychainAvailable(), ipc.keychainIsEnabled()])
      .then(([supp, en]) => setKeychainOption(supp && en))
      .catch(() => {});
  }, []);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!pw || busy) return;
    setBusy(true);
    setError(null);
    try {
      await ipc.unlock(pw);
      setPw("");
      onUnlocked();
    } catch (e) {
      if (isAppError(e) && e.kind === "WrongPassword") {
        setError(t("unlock_wrong_pw"));
      } else if (isAppError(e) && e.kind === "VaultCorrupt") {
        setError(t("unlock_corrupt"));
      } else {
        setError(isAppError(e) ? e.message : String(e));
      }
    } finally {
      setBusy(false);
    }
  }

  async function quickUnlock() {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await ipc.unlockWithKeychain();
      onUnlocked();
    } catch (e) {
      if (isAppError(e) && e.kind === "KeychainUnavailable") {
        setError(t("unlock_keychain_unavailable"));
      } else if (isAppError(e) && e.kind === "WrongPassword") {
        setError(t("unlock_keychain_mismatch"));
      } else {
        setError(isAppError(e) ? e.message : String(e));
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="screen centered">
      <form className="card" onSubmit={submit}>
        <h1>{t("unlock_title")}</h1>
        {reason === "idle" && (
          <p className="muted">{t("unlock_idle")}</p>
        )}

        {keychainOption && (
          <button type="button" className="secondary" onClick={quickUnlock} disabled={busy}>
            {t("unlock_keychain_btn")}
          </button>
        )}

        <label>
          {t("unlock_pw_label")}
          <input
            type="password"
            value={pw}
            onChange={(e) => setPw(e.target.value)}
            autoFocus
            autoComplete="current-password"
          />
        </label>

        {error && <div className="error">{error}</div>}

        <button type="submit" disabled={!pw || busy}>
          {busy ? t("unlock_unlocking") : t("unlock_btn")}
        </button>
      </form>
    </div>
  );
}
