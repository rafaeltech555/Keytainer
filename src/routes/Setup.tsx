import { useState } from "react";
import { ipc } from "../lib/ipc";
import { isAppError } from "../lib/types";

interface Props {
  onCreated: () => void;
}

export function Setup({ onCreated }: Props) {
  const [pw, setPw] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canSubmit =
    pw.length >= 8 && pw === confirm && !busy;

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      await ipc.createVault(pw);
      onCreated();
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="screen centered">
      <form className="card" onSubmit={submit}>
        <h1>歡迎使用 Keytainer</h1>
        <p className="muted">
          設定一個主密碼來加密你的本機保險庫。<br />
          <strong>主密碼遺失就無法復原</strong>，請選擇你能記住、且夠強的密碼。
        </p>

        <label>
          主密碼（至少 8 字元）
          <input
            type="password"
            value={pw}
            onChange={(e) => setPw(e.target.value)}
            autoFocus
            autoComplete="new-password"
          />
        </label>

        <label>
          再輸入一次
          <input
            type="password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            autoComplete="new-password"
          />
          {confirm.length > 0 && confirm !== pw && (
            <span className="error-inline">兩次輸入不一致</span>
          )}
        </label>

        {error && <div className="error">{error}</div>}

        <button type="submit" disabled={!canSubmit}>
          {busy ? "建立中…" : "建立保險庫"}
        </button>
      </form>
    </div>
  );
}
