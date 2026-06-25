// webui/src/types/api.ts (Phase 1.5 — 12 IPC contract locked, Phase 1.5 oracle fixes applied)
// @see openspec/changes/v0.1-general-credentials/spec.md REQ-VIS-001/002, REQ-PROV-008, REQ-THEME-001, REQ-CAT-001
// @see src-tauri/src/types.rs (Rust mirror — must stay field-by-field aligned)

// === Enums (V0.1 二态 / 三态) ===
export type Visibility = 'visible' | 'masked';  // REQ-VIS-002
export type Theme = 'dark' | 'light' | 'auto';  // REQ-THEME-001

// === Domain types ===
export interface ProviderField {
  id: number;
  provider_id: number;
  key: string;
  value: string;                       // REQ-VIS-002: V0.1 plaintext
  visibility: Visibility;
  sort_index: number;
  created_at: number;                  // unix epoch seconds
  updated_at: number;
}

export interface Provider {
  id: number;
  name: string;
  preset: string | null;               // null = Custom
  is_preset: boolean;
  category_id: number;
  pinned: boolean;
  notes: string | null;
  icon: string | null;
  icon_color: string | null;
  sort_index: number;
  created_at: number;
  updated_at: number;
  fields: ProviderField[];             // list responses include fields inline
}

export interface Category {
  id: number;
  name: string;
  is_default: boolean;
  sort_index: number;
  created_at: number;
  updated_at: number;
}

// QuotaSnapshot — per-preset shape (REQ-QUOTA-001~006 + REQ-QUOTA-DISPLAY-001):
// - 3 LLM: { total, used, remaining, unit, level?, reset_at? }
// - GitHub: { total, used, remaining, unit='req', level?, reset_at? }
// - PostgreSQL: { total=null, used, unit='GB', level? }  (no remaining, no reset_at)
// - Anthropic: AppError::ProviderQuotaUnsupported (no QuotaSnapshot returned)
// `fetched_at` is NOT in the wire shape — frontend uses TanStack Query staleTime.
export interface QuotaSnapshot {
  total: number | null;                // null = PostgreSQL or N/A
  used: number;                        // always present
  remaining?: number;                  // computed if total+used known; null for PostgreSQL
  unit: 'USD' | 'CNY' | 'req' | 'GB' | 'token';  // strict union (no `| string` escape hatch)
  level?: 'green' | 'amber' | 'red' | 'ruby';    // UI visual hint
  reset_at?: number;                   // unix epoch seconds, optional
}

// === AppError (mirrors src-tauri/src/error.rs — 11 codes) ===
export type AppErrorCode =
  | 'DATABASE'
  | 'IO'
  | 'SERDE'
  | 'INVALID_VISIBILITY'
  | 'INVALID_THEME'
  | 'PROVIDER_NOT_FOUND'
  | 'CATEGORY_NOT_FOUND'
  | 'CATEGORY_IS_DEFAULT'
  | 'PROVIDER_CANNOT_TEST'
  | 'PROVIDER_QUOTA_UNSUPPORTED'
  | 'HTTP';

export interface AppError {
  code: AppErrorCode;                  // literal union for exhaustiveness checks
  message: string;
}

// === 12 IPC command request/response types ===
// REQ-PROV-001/008 + REQ-CAT-001/002 + REQ-THEME-001 + REQ-QUOTA-001~006 + REQ-PROV-009
// All Rust commands use single-struct arg pattern (REQ-008, design.md §7).
// JS calls: invoke<Res>('cmd_name', req)  where req is the Request struct.

// list_providers
export type ListProvidersRequest = void;       // no filters in V0.1 (Phase 2A scope creep)
export type ListProvidersResponse = Provider[];

// get_provider
export interface GetProviderRequest { id: number; }
export type GetProviderResponse = Provider;

// add_provider
export interface AddProviderRequest {
  name: string;
  preset: string | null;               // null = Custom
  category_id: number;
  pinned?: boolean;
  notes?: string;
  icon?: string;
  icon_color?: string;
  // fields initial set on create (omit id/timestamps/server-managed fields)
  fields?: Array<Omit<ProviderField, 'id' | 'provider_id' | 'created_at' | 'updated_at'>>;
}
export type AddProviderResponse = Provider;

// update_provider (single-struct pattern matching design.md §7)
// JS: invoke('update_provider', { id, name?, category_id?, pinned?, notes?, icon?, icon_color?, fields? })
export interface UpdateProviderRequest {
  id: number;
  name?: string;
  category_id?: number;
  pinned?: boolean;
  notes?: string | null;
  icon?: string | null;
  icon_color?: string | null;
  // fields REPLACE-ALL semantics: server deletes fields not in this list, server sets id/timestamps.
  fields?: Array<Omit<ProviderField, 'id' | 'provider_id' | 'created_at' | 'updated_at'>>;
}
export type UpdateProviderResponse = Provider;

// delete_provider
export interface DeleteProviderRequest { id: number; }
export type DeleteProviderResponse = void;

// list_categories
export type ListCategoriesResponse = Category[];

// add_category
export interface AddCategoryRequest { name: string; }
export type AddCategoryResponse = Category;

// delete_category
export interface DeleteCategoryRequest { id: number; migrate_to: number; }
export type DeleteCategoryResponse = void;

// test_connection
export interface TestConnectionRequest { id: number; }
export type TestConnectionResponse = void;

// fetch_quota
export interface FetchQuotaRequest { id: number; }
export type FetchQuotaResponse = QuotaSnapshot;

// get_theme
export type GetThemeResponse = Theme;

// set_theme
export interface SetThemeRequest { theme: Theme; }
export type SetThemeResponse = void;

// pin_provider (Stage 5)
export interface PinProviderRequest { id: number; }
export type PinProviderResponse = void;

// unpin_provider
export interface UnpinProviderRequest { id: number; }
export type UnpinProviderResponse = void;

// quit_app
export type QuitAppResponse = void;

// set_manual_quota (Phase 3 — stores in quota_cache without touching provider notes)
export interface SetManualQuotaRequest { id: number; snapshot: QuotaSnapshot; }
export type SetManualQuotaResponse = void;