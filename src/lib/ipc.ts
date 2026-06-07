import { invoke } from "@tauri-apps/api/core";
import type { AuditReport, ItemInput, ItemSummary, Settings, TotpState, VaultItem } from "./types";

export const ipc = {
  vaultExists: () => invoke<boolean>("vault_exists"),
  isUnlocked: () => invoke<boolean>("is_unlocked"),

  createVault: (password: string) =>
    invoke<void>("create_vault", { password }),

  unlock: (password: string) =>
    invoke<void>("unlock", { password }),

  lock: () => invoke<void>("lock"),

  pingActivity: () => invoke<void>("ping_activity"),

  getSystemLocale: () => invoke<string>("get_system_locale"),

  changePassword: (current: string, newPassword: string) =>
    invoke<void>("change_password", { current, new: newPassword }),

  listItems: (query?: string, tag?: string) =>
    invoke<ItemSummary[]>("list_items", {
      query: query ?? null,
      tag: tag ?? null,
    }),

  listTags: () => invoke<string[]>("list_tags"),

  getItem: (id: string) => invoke<VaultItem>("get_item", { id }),

  addItem: (item: ItemInput) => invoke<string>("add_item", { item }),

  updateItem: (item: ItemInput) =>
    invoke<void>("update_item", { item }),

  deleteItem: (id: string) => invoke<void>("delete_item", { id }),

  computeTotp: (id: string) => invoke<TotpState>("compute_totp", { id }),

  copyPassword: (id: string) => invoke<void>("copy_password", { id }),

  copyTotp: (id: string) => invoke<void>("copy_totp", { id }),

  getSettings: () => invoke<Settings>("get_settings"),

  saveSettings: (newSettings: Settings) =>
    invoke<void>("save_settings", { newSettings }),

  generatePassword: (length: number, includeSymbols: boolean) =>
    invoke<string>("generate_password", { length, includeSymbols }),

  auditPasswords: () => invoke<AuditReport>("audit_passwords"),

  exportVault: (path: string, password: string) =>
    invoke<void>("export_vault", { path, password }),

  importVault: (path: string, password: string) =>
    invoke<{ added: number; updated: number }>("import_vault", { path, password }),

  keychainAvailable: () => invoke<boolean>("keychain_available"),
  keychainIsEnabled: () => invoke<boolean>("keychain_is_enabled"),
  keychainEnable: () => invoke<void>("keychain_enable"),
  keychainDisable: () => invoke<void>("keychain_disable"),
  unlockWithKeychain: () => invoke<void>("unlock_with_keychain"),
};
