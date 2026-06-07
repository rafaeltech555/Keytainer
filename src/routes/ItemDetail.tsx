import { useEffect, useRef, useState } from "react";
import { ipc } from "../lib/ipc";
import type { GenOptions, ItemInput, VaultItem, TotpEntry, TotpAlg, PasswordHistoryEntry } from "../lib/types";
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
  const [history, setHistory] = useState<PasswordHistoryEntry[]>([]);
  const [showHistory, setShowHistory] = useState(false);
  const [copiedIdx, setCopiedIdx] = useState<number | null>(null);
  const histCopiedTimer = useRef<number | null>(null);
  const [genOpen, setGenOpen] = useState(false);
  const [gen, setGen] = useState<GenOptions>({
    mode: "chars",
    length: 20,
    symbols: true,
    avoid_ambiguous: false,
    words: 5,
    separator: "-",
    capitalize: true,
    number: true,
  });
  function setGenOpt<K extends keyof GenOptions>(k: K, v: GenOptions[K]) {
    setGen((g) => ({ ...g, [k]: v }));
  }

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
        setHistory(it.password_history ?? []);
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
    const pw = await ipc.generatePassword(gen);
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

  async function copyHistory(index: number) {
    if (!savedItemId) return;
    try {
      await ipc.copyHistoryPassword(savedItemId, index);
      setCopiedIdx(index);
      if (histCopiedTimer.current) window.clearTimeout(histCopiedTimer.current);
      histCopiedTimer.current = window.setTimeout(() => setCopiedIdx(null), 1500);
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    }
  }

  function restoreHistory(entry: PasswordHistoryEntry) {
    patch("password", entry.password);
    setConfirmWeak(false);
    setShowPw(true);
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
            <button type="button" onClick={() => setGenOpen((o) => !o)}>
              {t("gen_panel_toggle")}
            </button>
          </div>
          <StrengthMeter password={form.password} />
          {confirmWeak && (
            <span className="error-inline">{t("detail_pw_weak_warn")}</span>
          )}
        </label>

        {genOpen && (
          <div className="gen-panel">
            <div className="gen-seg">
              <button
                type="button"
                className={gen.mode === "chars" ? "active" : ""}
                onClick={() => setGenOpt("mode", "chars")}
              >
                {t("gen_mode_random")}
              </button>
              <button
                type="button"
                className={gen.mode === "passphrase" ? "active" : ""}
                onClick={() => setGenOpt("mode", "passphrase")}
              >
                {t("gen_mode_passphrase")}
              </button>
            </div>

            {gen.mode === "chars" ? (
              <>
                <label className="gen-row">
                  <span>{t("gen_length")}</span>
                  <input
                    type="range"
                    min={8}
                    max={64}
                    value={gen.length}
                    onChange={(e) => setGenOpt("length", Number(e.target.value))}
                  />
                  <span className="gen-val">{gen.length}</span>
                </label>
                <label className="gen-row">
                  <span>{t("gen_symbols")}</span>
                  <input
                    type="checkbox"
                    checked={gen.symbols}
                    onChange={(e) => setGenOpt("symbols", e.target.checked)}
                  />
                </label>
                <label className="gen-row">
                  <span>{t("gen_avoid_ambiguous")}</span>
                  <input
                    type="checkbox"
                    checked={gen.avoid_ambiguous}
                    onChange={(e) => setGenOpt("avoid_ambiguous", e.target.checked)}
                  />
                </label>
              </>
            ) : (
              <>
                <label className="gen-row">
                  <span>{t("gen_words")}</span>
                  <input
                    type="range"
                    min={3}
                    max={12}
                    value={gen.words}
                    onChange={(e) => setGenOpt("words", Number(e.target.value))}
                  />
                  <span className="gen-val">{gen.words}</span>
                </label>
                <label className="gen-row">
                  <span>{t("gen_separator")}</span>
                  <select
                    value={gen.separator}
                    onChange={(e) => setGenOpt("separator", e.target.value)}
                  >
                    <option value="-">-</option>
                    <option value=".">.</option>
                    <option value="_">_</option>
                    <option value=" ">{t("gen_sep_space")}</option>
                  </select>
                </label>
                <label className="gen-row">
                  <span>{t("gen_capitalize")}</span>
                  <input
                    type="checkbox"
                    checked={gen.capitalize}
                    onChange={(e) => setGenOpt("capitalize", e.target.checked)}
                  />
                </label>
                <label className="gen-row">
                  <span>{t("gen_number")}</span>
                  <input
                    type="checkbox"
                    checked={gen.number}
                    onChange={(e) => setGenOpt("number", e.target.checked)}
                  />
                </label>
              </>
            )}

            <button type="button" className="gen-go" onClick={generate}>
              {t("gen_generate")}
            </button>
          </div>
        )}

        {history.length > 0 && (
          <div className="history-block">
            <div className="history-head">
              <span>{t("detail_history_section")}</span>
              <button
                type="button"
                className="secondary"
                onClick={() => setShowHistory((s) => !s)}
              >
                {showHistory ? t("detail_history_hide") : t("detail_history_show")}
              </button>
            </div>
            <ul className="history-list">
              {history.map((entry, i) => (
                <li key={i} className="history-row">
                  <div className="history-main">
                    <span className="history-pw">
                      {showHistory ? entry.password : "••••••••"}
                    </span>
                    <span className="history-date">
                      {new Date(entry.changed_at * 1000).toLocaleDateString()}
                    </span>
                  </div>
                  <div className="history-actions">
                    <button type="button" onClick={() => copyHistory(i)}>
                      {copiedIdx === i ? t("detail_history_copied") : t("detail_history_copy")}
                    </button>
                    <button type="button" onClick={() => restoreHistory(entry)}>
                      {t("detail_history_restore")}
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        )}

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

