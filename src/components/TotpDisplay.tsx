import { useEffect, useRef, useState } from "react";
import { ipc } from "../lib/ipc";
import type { TotpState } from "../lib/types";
import { isAppError } from "../lib/types";

interface Props {
  itemId: string;
  showCode: boolean;
  onCopy?: () => void;
}

export function TotpDisplay({ itemId, showCode, onCopy }: Props) {
  const [state, setState] = useState<TotpState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const copiedTimer = useRef<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function tick() {
      try {
        const s = await ipc.computeTotp(itemId);
        if (!cancelled) {
          setState(s);
          setError(null);
        }
      } catch (e) {
        if (!cancelled) setError(isAppError(e) ? e.message : String(e));
      }
    }

    void tick();
    const id = window.setInterval(tick, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [itemId]);

  async function copy() {
    try {
      await ipc.copyTotp(itemId);
      setCopied(true);
      onCopy?.();
      if (copiedTimer.current) window.clearTimeout(copiedTimer.current);
      copiedTimer.current = window.setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    }
  }

  if (error) return <div className="error">{error}</div>;
  if (!state) return <div className="muted">計算中…</div>;

  const grouped =
    state.code.length === 6
      ? `${state.code.slice(0, 3)} ${state.code.slice(3)}`
      : state.code;
  const ratio = state.remaining_seconds / Math.max(state.period, 1);

  return (
    <div className="totp-display">
      <div className="totp-code-row">
        <span className="totp-code">{showCode ? grouped : "● ● ● ● ● ●"}</span>
        <CountdownRing ratio={ratio} seconds={state.remaining_seconds} />
        <button type="button" onClick={copy}>
          {copied ? "已複製 ✓" : "複製"}
        </button>
      </div>
    </div>
  );
}

function CountdownRing({ ratio, seconds }: { ratio: number; seconds: number }) {
  // SVG ring: stroke-dashoffset shrinks as ratio shrinks.
  const r = 14;
  const c = 2 * Math.PI * r;
  const offset = c * (1 - ratio);
  // Color shifts to amber under 10s, red under 5s.
  const stroke = seconds <= 5 ? "var(--danger)" : seconds <= 10 ? "#f5a524" : "var(--accent)";
  return (
    <svg className="totp-ring" width="36" height="36" viewBox="0 0 36 36">
      <circle cx="18" cy="18" r={r} fill="none" stroke="var(--surface-2)" strokeWidth="3" />
      <circle
        cx="18"
        cy="18"
        r={r}
        fill="none"
        stroke={stroke}
        strokeWidth="3"
        strokeDasharray={c}
        strokeDashoffset={offset}
        strokeLinecap="round"
        transform="rotate(-90 18 18)"
        style={{ transition: "stroke-dashoffset 1s linear, stroke 0.3s" }}
      />
      <text x="18" y="22" textAnchor="middle" fontSize="11" fill="currentColor">
        {seconds}
      </text>
    </svg>
  );
}
