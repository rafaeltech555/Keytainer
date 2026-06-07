import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";
import type { AuditReport } from "../lib/types";
import { isAppError } from "../lib/types";
import { useT } from "../lib/i18n";

interface Props {
  onBack: () => void;
  onSelect: (id: string) => void;
}

export function Audit({ onBack, onSelect }: Props) {
  const t = useT();
  const [report, setReport] = useState<AuditReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  function load() {
    setLoading(true);
    setError(null);
    ipc
      .auditPasswords()
      .then(setReport)
      .catch((e) => setError(isAppError(e) ? e.message : String(e)))
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    load();
  }, []);

  const clean =
    report !== null && report.reused.length === 0 && report.weak.length === 0;

  return (
    <div className="screen audit-screen">
      <header className="audit-header">
        <button className="secondary" onClick={onBack}>{t("back")}</button>
        <h2>{t("audit_title")}</h2>
        <button className="secondary" onClick={load} disabled={loading}>
          {t("audit_rescan")}
        </button>
      </header>

      {error && <div className="error">{error}</div>}

      {loading ? (
        <p className="muted">{t("audit_loading")}</p>
      ) : report === null ? null : clean ? (
        <p className="muted audit-none">{t("audit_none")}</p>
      ) : (
        <>
          <p className="muted audit-summary">
            {t("audit_summary", {
              reused: String(report.reused.length),
              weak: String(report.weak.length),
            })}
          </p>

          {report.reused.length > 0 && (
            <section>
              <h3 className="audit-section">{t("audit_reused_section")}</h3>
              {report.reused.map((group, gi) => (
                <div key={gi} className="audit-group">
                  <div className="audit-group-head">
                    {t("audit_group_count", { count: String(group.items.length) })}
                  </div>
                  {group.items.map((it) => (
                    <button
                      key={it.id}
                      type="button"
                      className="audit-row"
                      onClick={() => onSelect(it.id)}
                    >
                      <div className="audit-row-main">
                        <div className="audit-row-title">
                          {it.site_name || t("list_unnamed")}
                        </div>
                        <div className="audit-row-sub">{it.username}</div>
                      </div>
                    </button>
                  ))}
                </div>
              ))}
            </section>
          )}

          {report.weak.length > 0 && (
            <section>
              <h3 className="audit-section">{t("audit_weak_section")}</h3>
              <div className="audit-group">
                {report.weak.map((w) => (
                  <button
                    key={w.item.id}
                    type="button"
                    className="audit-row"
                    onClick={() => onSelect(w.item.id)}
                  >
                    <div className="audit-row-main">
                      <div className="audit-row-title">
                        {w.item.site_name || t("list_unnamed")}
                      </div>
                      <div className="audit-row-sub">{w.item.username}</div>
                    </div>
                    <span className="audit-pill weak">{t("audit_weak_pill")}</span>
                  </button>
                ))}
              </div>
            </section>
          )}
        </>
      )}
    </div>
  );
}
