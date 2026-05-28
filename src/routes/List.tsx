import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";
import type { ItemSummary } from "../lib/types";

interface Props {
  onSelect: (id: string | "new") => void;
  onLock: () => void;
  onSettings: () => void;
  refreshKey: number;
}

export function List({ onSelect, onLock, onSettings, refreshKey }: Props) {
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
          <button onClick={() => onSelect("new")}>＋ 新增</button>
          <button className="secondary" onClick={onSettings}>⚙</button>
          <button className="secondary" onClick={handleLock}>🔒 鎖定</button>
        </div>
      </header>

      <input
        className="search"
        placeholder="搜尋網站、帳號、標籤…"
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
            全部
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
        <p className="muted">載入中…</p>
      ) : items.length === 0 ? (
        <div className="empty">
          <p className="muted">
            {query || activeTag
              ? "沒有符合的項目"
              : "還沒有任何項目，點右上「新增」開始"}
          </p>
        </div>
      ) : (
        <ul className="item-list">
          {items.map((it) => (
            <li key={it.id}>
              <button className="row" onClick={() => onSelect(it.id)}>
                <div className="row-main">
                  <div className="row-title">{it.site_name || "(未命名)"}</div>
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
