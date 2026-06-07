import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";
import type { ItemSummary } from "../lib/types";
import { useT } from "../lib/i18n";

interface Props {
  onSelect: (id: string | "new") => void;
  onLock: () => void;
  onSettings: () => void;
  onAudit: () => void;
  refreshKey: number;
}

export function List({ onSelect, onLock, onSettings, onAudit, refreshKey }: Props) {
  const t = useT();
  const [items, setItems] = useState<ItemSummary[]>([]);
  const [tags, setTags] = useState<string[]>([]);
  const [activeTag, setActiveTag] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ipc.listTags().then(setTags).catch(() => {});
  }, [refreshKey]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    ipc.listItems(query, activeTag ?? undefined)
      .then((items) => {
        if (!cancelled) setItems(items);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e?.message ?? e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [query, activeTag, refreshKey]);

  async function handleLock() {
    await ipc.lock();
    onLock();
  }

  return (
    <div className="screen list-screen">
      <header className="list-header">
        <h1>Keytainer</h1>
        <div className="header-actions">
          <button onClick={() => onSelect("new")}>{t("list_add")}</button>
          <button className="secondary" onClick={onAudit}>{t("list_audit_btn")}</button>
          <button className="secondary" onClick={onSettings}>⚙</button>
          <button className="secondary" onClick={handleLock}>{t("list_lock")}</button>
        </div>
      </header>

      <input
        className="search"
        placeholder={t("list_search")}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />

      {tags.length > 0 && (
        <div className="tag-bar">
          <button
            type="button"
            className={`chip ${activeTag === null ? "active" : ""}`}
            onClick={() => setActiveTag(null)}
          >
            {t("list_all")}
          </button>
          {tags.map((t) => (
            <button
              key={t}
              type="button"
              className={`chip ${activeTag === t ? "active" : ""}`}
              onClick={() => setActiveTag(activeTag === t ? null : t)}
            >
              {t}
            </button>
          ))}
        </div>
      )}

      {error && <div className="error">{error}</div>}

      {loading ? (
        <p className="muted">{t("loading")}</p>
      ) : items.length === 0 ? (
        <div className="empty">
          <p className="muted">
            {query || activeTag ? t("list_no_match") : t("list_empty")}
          </p>
        </div>
      ) : (
        <ul className="item-list">
          {items.map((it) => (
            <li key={it.id}>
              <button className="row" onClick={() => onSelect(it.id)}>
                <div className="row-main">
                  <div className="row-title">{it.site_name || t("list_unnamed")}</div>
                  <div className="row-sub">{it.username}</div>
                </div>
                <div className="row-side">
                  {it.has_totp && <span className="badge">2FA</span>}
                  {it.tags.map((t) => (
                    <span key={t} className="badge tag">{t}</span>
                  ))}
                </div>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
