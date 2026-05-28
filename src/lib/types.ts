export type TotpAlg = "SHA1" | "SHA256" | "SHA512";

export interface TotpEntry {
  secret: string;
  algorithm: TotpAlg;
  digits: number;
  period: number;
  issuer?: string | null;
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
