import { useEffect, useRef, useState } from "react";
import { ipc } from "../lib/ipc";
import type { ItemInput, VaultItem, TotpEntry, TotpAlg } from "../lib/types";
import { isAppError } from "../lib/types";
import { TotpDisplay } from "../components/TotpDisplay";
import { useT, type TKey } from "../lib/i18n";
import { scorePassword, MIN_MASTER_SCORE } from "../lib/strength";
import { StrengthMeter } from "../components/StrengthMeter";

interface Props {
  itemId: string | "new";
  onClose: () => void;
  onSaved: () => void;
  onDeleted: () => void;
}

const empty: ItemInput = {
  site_name: "",
  username: "",
  password: "",
  url: "",
  notes: "",
  tags: [],
};

function defaultTotp(): TotpEntry {
  return { secret: "", algorithm: "SHA1", digits: 6, period: 30 };
}

export function ItemDetail({ itemId, onClose, onSaved, onDeleted }: Props) {
  const t = useT();
  const isNew = itemId === "new";
  const [form, setForm] = useState<ItemInput>(empty);
  const [tagsInput, setTagsInput] = useState("");
  const [showPw, setShowPw] = useState(false);
  const [confirmWeak, setConfirmWeak] = useState(false);
  const [loading, setLoading] = useState(!isNew);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const savedItemId: string | null = isNew ? null : (itemId as string);
  const [pwCopied, setPwCopied] = useState(false);
  const pwCopiedTimer = useRef<number | null>(null);
  const [showTotpCode, setShowTotpCode] = useState(true);

  useEffect(() => {
    ipc.getSettings()
      .then((s) => setShowTotpCode(s.show_totp_code))
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (isNew) return;
    let cancelled = false;
    setLoading(true);
    ipc.getItem(itemId as string)
      .then((it: VaultItem) => {
        if (cancelled) return;
        setForm({
          id: it.id,
          site_name: it.site_name,
          username: it.username,
          password: it.password,
          totp: it.totp ?? null,
          url: it.url ?? "",
          notes: it.notes ?? "",
          tags: it.tags,
        });
        setTagsInput(it.tags.join(", "));
      })
      .catch((e) => setError(isAppError(e) ? e.message : String(e)))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [itemId, isNew]);

  function patch<K extends keyof ItemInput>(key: K, value: ItemInput[K]) {
    setForm((f) => ({ ...f, [key]: value }));
  }

  function commitTags(raw: string) {
    setTagsInput(raw);
    const tags = raw
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean);
    patch("tags", tags);
  }

  async function generate() {
    const pw = await ipc.generatePassword(20, true);
    patch("password", pw);
    setConfirmWeak(false);
    setShowPw(true);
  }

  async function copyPassword() {
    if (!savedItemId) return;
    try {
      await ipc.copyPassword(savedItemId);
      setPwCopied(true);
      if (pwCopiedTimer.current) window.clearTimeout(pwCopiedTimer.current);
      pwCopiedTimer.current = window.setTimeout(() => setPwCopied(false), 1500);
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    }
  }

  async function save(e: React.FormEvent) {
    e.preventDefault();
    if (busy) return;
    const pw = form.password;
    if (pw && scorePassword(pw) < MIN_MASTER_SCORE && !confirmWeak) {
      setConfirmWeak(true);
      return; // show the warning; a second click confirms
    }
    setBusy(true);
    setError(null);
    try {
      const payload: ItemInput = {
        ...form,
        url: form.url || null,
        notes: form.notes || null,
        totp: form.totp && form.totp.secret ? form.totp : null,
      };
      if (isNew) {
        await ipc.addItem(payload);
      } else {
        await ipc.updateItem(payload);
      }
      onSaved();
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (isNew) return;
    if (!confirm(t("detail_confirm_delete"))) return;
    setBusy(true);
    try {
      await ipc.deleteItem(itemId as string);
      onDeleted();
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
      setBusy(false);
    }
  }

  const hasTotp = form.totp !== null && form.totp !== undefined;

  if (loading) return <div className="screen centered"><p>{t("loading")}</p></div>;

  return (
    <div className="screen detail-screen">
      <header className="detail-header">
        <button className="secondary" onClick={onClose}>{t("back")}</button>
        <h2>{isNew ? t("detail_new") : form.site_name || t("list_unnamed")}</h2>
        {!isNew && (
          <button className="danger" onClick={remove} disabled={busy}>
            {t("delete")}
          </button>
        )}
      </header>

      {/* For saved items: action chips for one-click copy + live TOTP */}
      {savedItemId && (
        <div className="quick-actions">
          <button type="button" onClick={copyPassword}>
            {pwCopied ? t("detail_copied_pw") : t("detail_copy_pw")}
          </button>
          {hasTotp && (
            <TotpDisplay itemId={savedItemId} showCode={showTotpCode} />
          )}
        </div>
      )}

      <form onSubmit={save} className="detail-form">
        <label>
          {t("detail_site")}
          <input
            value={form.site_name}
            onChange={(e) => patch("site_name", e.target.value)}
            autoFocus
            required
          />
        </label>

        <label>
          {t("detail_username")}
          <input
            value={form.username}
            onChange={(e) => patch("username", e.target.value)}
          />
        </label>

        <label>
          {t("detail_password")}
          <div className="pw-row">
            <input
              type={showPw ? "text" : "password"}
              value={form.password}
              onChange={(e) => {
                patch("password", e.target.value);
                setConfirmWeak(false);
              }}
            />
            <button type="button" onClick={() => setShowPw((s) => !s)}>
              {showPw ? t("detail_hide") : t("detail_show")}
            </button>
            <button type="button" onClick={generate}>{t("detail_generate")}</button>
          </div>
          <StrengthMeter password={form.password} />
          {confirmWeak && (
            <span className="error-inline">{t("detail_pw_weak_warn")}</span>
          )}
        </label>

        <label>
          {t("detail_url")}
          <input
            type="url"
            value={form.url ?? ""}
            onChange={(e) => patch("url", e.target.value)}
          />
        </label>

        <fieldset className="totp-block">
          <legend>{t("detail_2fa")}</legend>
          {hasTotp ? (
            <TotpFields
              totp={form.totp!}
              onChange={(t) => patch("totp", t)}
              onRemove={() => patch("totp", null)}
            />
          ) : (
            <button
              type="button"
              onClick={() => patch("totp", defaultTotp())}
            >
              {t("detail_add_2fa")}
            </button>
          )}
        </fieldset>

        <label>
          {t("detail_tags")}
          <input
            value={tagsInput}
            onChange={(e) => commitTags(e.target.value)}
            placeholder={t("detail_tags_ph")}
          />
        </label>

        <label>
          {t("detail_notes")}
          <textarea
            value={form.notes ?? ""}
            onChange={(e) => patch("notes", e.target.value)}
            rows={4}
          />
        </label>

        {error && <div className="error">{error}</div>}

        <div className="form-actions">
          <button type="submit" disabled={busy || !form.site_name}>
            {busy
              ? t("detail_saving")
              : confirmWeak
                ? t("detail_save_weak")
                : t("save")}
          </button>
          <button type="button" className="secondary" onClick={onClose}>
            {t("cancel")}
          </button>
        </div>
      </form>
    </div>
  );
}

interface TotpFieldsProps {
  totp: TotpEntry;
  onChange: (t: TotpEntry) => void;
  onRemove: () => void;
}

function TotpFields({ totp, onChange, onRemove }: TotpFieldsProps) {
  const t = useT();
  function patch<K extends keyof TotpEntry>(key: K, value: TotpEntry[K]) {
    onChange({ ...totp, [key]: value });
  }
  return (
    <div className="totp-fields">
      <label>
        {t("totp_secret" as TKey)}
        <input
          value={totp.secret}
          onChange={(e) => patch("secret", e.target.value)}
          placeholder="JBSWY3DPEHPK3PXP"
          spellCheck={false}
        />
      </label>
      <div className="totp-row">
        <label>
          {t("totp_algorithm")}
          <select
            value={totp.algorithm}
            onChange={(e) => patch("algorithm", e.target.value as TotpAlg)}
          >
            <option value="SHA1">SHA-1</option>
            <option value="SHA256">SHA-256</option>
            <option value="SHA512">SHA-512</option>
          </select>
        </label>
        <label>
          {t("totp_digits")}
          <input
            type="number"
            min={6}
            max={10}
            value={totp.digits}
            onChange={(e) => patch("digits", Number(e.target.value))}
          />
        </label>
        <label>
          {t("totp_period")}
          <input
            type="number"
            min={15}
            max={120}
            value={totp.period}
            onChange={(e) => patch("period", Number(e.target.value))}
          />
        </label>
      </div>
      <button type="button" className="secondary" onClick={onRemove}>
        {t("totp_remove")}
      </button>
    </div>
  );
}

