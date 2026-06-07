import { useMemo } from "react";
import { scorePassword } from "../lib/strength";
import { useT, type TKey } from "../lib/i18n";

const SEGMENTS = 4;

interface Props {
  password: string;
}

/** Live password-strength bar + label. Renders nothing for an empty password. */
export function StrengthMeter({ password }: Props) {
  const t = useT();
  const score = useMemo(() => scorePassword(password), [password]);
  if (!password) return null;

  // zxcvbn scores 0..4; always light at least one segment so a score of 0
  // still reads as "something was measured".
  const filled = Math.max(1, score);

  return (
    <div className={`strength-meter level-${score}`} data-testid="strength-meter">
      <div className="strength-bar" aria-hidden="true">
        {Array.from({ length: SEGMENTS }, (_, i) => (
          <span key={i} className={`strength-seg${i < filled ? " filled" : ""}`} />
        ))}
      </div>
      <span className="strength-label">
        {t("strength_prefix")} {t(`strength_label_${score}` as TKey)}
      </span>
    </div>
  );
}
