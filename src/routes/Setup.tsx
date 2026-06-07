import { useState } from "react";
import { ipc } from "../lib/ipc";
import { isAppError } from "../lib/types";
import { useT } from "../lib/i18n";
import { scorePassword, MIN_MASTER_SCORE } from "../lib/strength";
import { StrengthMeter } from "../components/StrengthMeter";

interface Props {
  onCreated: () => void;
}

export function Setup({ onCreated }: Props) {
  const t = useT();
  const [pw, setPw] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const score = scorePassword(pw);
  const tooWeak = pw.length >= 8 && score < MIN_MASTER_SCORE;
  const canSubmit =
    pw.length >= 8 && score >= MIN_MASTER_SCORE && pw === confirm && !busy;

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
        <h1>{t("setup_title")}</h1>
        <p className="muted">
          {t("setup_intro_1")}<br />
          {t("setup_intro_2")}
        </p>

        <label>
          {t("setup_pw_label")}
          <input
            type="password"
            value={pw}
            onChange={(e) => setPw(e.target.value)}
            autoFocus
            autoComplete="new-password"
          />
          <StrengthMeter password={pw} />
          {tooWeak && (
            <span className="error-inline">{t("pw_too_weak")}</span>
          )}
        </label>

        <label>
          {t("setup_confirm_label")}
          <input
            type="password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            autoComplete="new-password"
          />
          {confirm.length > 0 && confirm !== pw && (
            <span className="error-inline">{t("setup_mismatch")}</span>
          )}
        </label>

        {error && <div className="error">{error}</div>}

        <button type="submit" disabled={!canSubmit}>
          {busy ? t("setup_creating") : t("setup_create_btn")}
        </button>
      </form>
    </div>
  );
}
