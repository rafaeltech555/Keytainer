export type TotpAlg = "SHA1" | "SHA256" | "SHA512";

export interface TotpEntry {
  secret: string;
  algorithm: TotpAlg;
  digits: number;
  period: number;
  issuer?: string | null;
}

export interface PasswordHistoryEntry {
  password: string;
  changed_at: number;
}

export interface VaultItem {
  id: string;
  site_name: string;
  username: string;
  password: string;
  totp?: TotpEntry | null;
  url?: string | null;
  notes?: string | null;
  tags: string[];
  password_history?: PasswordHistoryEntry[];
  created_at: number;
  updated_at: number;
}

export interface ItemSummary {
  id: string;
  site_name: string;
  username: string;
  url?: string | null;
  tags: string[];
  has_totp: boolean;
  updated_at: number;
}

export interface ItemInput {
  id?: string | null;
  site_name: string;
  username: string;
  password: string;
  totp?: TotpEntry | null;
  url?: string | null;
  notes?: string | null;
  tags: string[];
}

export type AppErrorKind =
  | "WrongPassword"
  | "VaultCorrupt"
  | "Locked"
  | "NotInitialised"
  | "AlreadyExists"
  | "ItemNotFound"
  | "InvalidTotpSecret"
  | "KeychainUnavailable"
  | "Clipboard"
  | "Crypto"
  | "Io"
  | "Serde";

export interface AppError {
  kind: AppErrorKind;
  message: string;
}

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    "message" in e
  );
}

export interface TotpState {
  code: string;
  remaining_seconds: number;
  period: number;
}

export interface Settings {
  auto_lock_seconds: number;
  clipboard_clear_seconds: number;
  show_totp_code: boolean;
  /** "system" (follow OS), "en", or "zh-TW". */
  locale: string;
}

export interface AuditItemRef {
  id: string;
  site_name: string;
  username: string;
}
export interface ReuseGroup {
  items: AuditItemRef[];
}
export interface WeakItem {
  item: AuditItemRef;
  score: number;
}
export interface AuditReport {
  reused: ReuseGroup[];
  weak: WeakItem[];
}
