import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";
import { isAppError } from "../lib/types";

interface Props {
  onUnlocked: () => void;
  reason?: "idle" | "manual";
}

export function Unlock({ onUnlocked, reason }: Props) {
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
        setError("主密碼錯誤");
      } else if (isAppError(e) && e.kind === "VaultCorrupt") {
        setError("保險庫檔案損毀或非 Keytainer 檔案");
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
        setError("Keychain 無法存取，請改用主密碼");
      } else if (isAppError(e) && e.kind === "WrongPassword") {
        setError("Keychain 內的金鑰跟保險庫對不起來，請改用主密碼");
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
        <h1>解鎖 Keytainer</h1>
        {reason === "idle" && (
          <p className="muted">已自動鎖定（閒置過久）</p>
        )}

        {keychainOption && (
          <button type="button" className="secondary" onClick={quickUnlock} disabled={busy}>
            🔑 用系統 keychain 一鍵解鎖
          </button>
        )}

        <label>
          主密碼
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
          {busy ? "解鎖中…" : "解鎖"}
        </button>
      </form>
    </div>
  );
}
