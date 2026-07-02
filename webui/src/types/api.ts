// webui/src/types/api.ts (Phase 1.5 -- 12 IPC contract locked, Phase 1.5 oracle fixes applied)
// @see openspec/changes/v0.1-general-credentials/spec.md REQ-VIS-001/002, REQ-PROV-008, REQ-THEME-001, REQ-CAT-001
// @see src-tauri/src/types.rs (Rust mirror -- must stay field-by-field aligned)

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

// QuotaSnapshot -- per-preset shape (REQ-QUOTA-001~006 + REQ-QUOTA-DISPLAY-001):
// - 3 LLM: { total, used, remaining, unit, level?, reset_at? }
// - GitHub: { total, used, remaining, unit='req', level?, reset_at? }
// - PostgreSQL: { total=null, used, unit='GB', level? }  (no remaining, no reset_at)
// - Anthropic: AppError::ProviderQuotaUnsupported (no QuotaSnapshot returned)
// `fetched_at` is NOT in the wire shape -- frontend uses TanStack Query staleTime.
export interface QuotaSnapshot {
  total: number | null;
  used: number;
  remaining?: number;
  unit: 'USD' | 'CNY' | 'req' | 'GB' | 'token' | string;
  level?: 'green' | 'amber' | 'red' | 'ruby' | string;
  reset_at?: number;
}

// === AppError (mirrors src-tauri/src/error.rs -- 15 codes) ===
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
  | 'HTTP'
  | 'TOKEN_USAGE_INVALID_FORMAT'
  | 'TOKEN_USAGE_DUPLICATE'
  | 'TOKEN_USAGE_PRICING_NOT_FOUND'
  | 'ACTION_VALIDATION'
  | 'ACTION_NOT_FOUND';

export interface AppError {
  code: AppErrorCode;                  // literal union for exhaustiveness checks
  message: string;
}

// === 12 IPC command request/response types ===
// REQ-PROV-001/008 + REQ-CAT-001/002 + REQ-THEME-001 + REQ-QUOTA-001~006 + REQ-PROV-009
// All Rust commands use single-struct arg pattern (REQ-008, design.md §7).
// JS calls: invoke<Res>('cmd_name', req)  where req is the Request struct.

// list_providers
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

// fetch_quota
export interface FetchQuotaRequest { id: number; }
export type FetchQuotaResponse = QuotaSnapshot;

// get_theme
export type GetThemeResponse = Theme;

// set_theme
export interface SetThemeRequest { theme: Theme; }
export type SetThemeResponse = void;

// set_manual_quota (Phase 3 -- stores in quota_cache without touching provider notes)
export interface SetManualQuotaRequest { id: number; snapshot: QuotaSnapshot; }
export type SetManualQuotaResponse = void;

// === Token Usage types (REQ-TOKEN-001.2/001.3/002) ===
//
// CONTRACT: field names are SNAKE_CASE to match Rust serde default output.
// If you add `#[serde(rename_all = "camelCase")]` on any Rust struct in
// `src-tauri/src/commands/token_usage.rs`, these field names must change
// in lockstep or all IPC calls will return undefined fields at runtime.
// See src-tauri/src/commands/token_usage.rs UsageFilterIpc + UsageSummaryResponse.

// TokenBreakdown -- REQ-TOKEN-001.2 usage_details
export interface TokenBreakdown {
  input?: number;
  output?: number;
  cache_read?: number;       // cache_read_input_tokens
  cache_creation?: number;  // cache_creation_input_tokens
  reasoning?: number;       // reasoning_tokens
}

// CostBreakdown -- REQ-TOKEN-001.3 cost_details
export interface CostBreakdown {
  input?: number;
  output?: number;
  cache_read?: number;
  total?: number;
}

// UsageRecordInput -- what the frontend sends to record_usage
export interface UsageRecordInput {
  occurred_at: string;
  finished_at?: string;
  latency_ms?: number;
  provider: string;
  model: string;
  agent_type?: string;
  user_id?: string;
  session_id?: string;
  observation_type?: string;
  status?: string;
  error_code?: string;
  cache_hit?: number;
  usage_details: TokenBreakdown;
  cost_details?: CostBreakdown;
  pricing_version?: string;
  messages?: string;
  response?: string;
  tags?: string[];
}

// UsageRecord -- returned by record_usage, list_usage_records
export interface UsageRecord extends UsageRecordInput {
  id: string;
}

// === Real-time tick (Bug #3 fix 2026-06-29) ===
//
// Mirrors `services::incremental_import::TokenUsageTickPayload`.  Emitted by
// the Rust file watcher after each successful incremental JSONL append so
// the frontend can refresh KPI cards / heatmap / popover (Task 2) without
// polling.  Field names are snake_case per the convention documented at
// the top of this file (api.ts:187-191).

export interface TokenUsageTickPayload {
  agent_type: string;
  imported: number;
  skipped: number;
  latest_at: number | null;
  total_today_tokens: number;
  total_today_cost_usd: number;
}

// UsageFilter -- shared by list_usage_records and get_usage_summary
export interface UsageFilter {
  start_date?: string;
  end_date?: string;
  agent_type?: string;
  model?: string;
  provider?: string;
  status?: string;
}

// AgentPair -- part of UsageSummary
export interface AgentPair {
  agent_type: string;
  model: string;
  provider: string;
  total_tokens: number;
  total_cost_usd: number;
  request_count: number;
  token_breakdown: {
    input?: number;
    output?: number;
    cache_read?: number;
    cache_creation?: number;
    reasoning?: number;
  };
}

// DailySeriesPoint -- part of UsageSummary
export interface DailySeriesPoint {
  date: string;
  total_tokens: number;
  total_cost_usd: number;
  request_count: number;
}

// UsageSummary -- returned by get_usage_summary (REQ-TOKEN-002.1)
export interface UsageSummary {
  total_tokens: number;
  total_cost_usd: number;
  total_requests: number;
  agent_pairs: AgentPair[];
  daily_series: DailySeriesPoint[];
}

// ImportResult -- returned by import_usage
export interface ImportResult {
  imported: number;
  skipped: number;
  errors: string[];
}

// ImportFormat -- union literal
export type ImportFormat = 'jsonl' | 'csv';

// PricingEntry -- returned by get_pricing
export interface PricingEntry {
  model: string;
  input_cost_per_token: number;
  output_cost_per_token: number;
  cache_read_cost?: number;
  cache_creation_cost?: number;
  supports_reasoning?: boolean;
}

// PaginatedResponse<T> -- standard pagination shape.
// Bug #2 fix 2026-06-29: field names are snake_case to match Rust
// `PaginatedResponseIpc` (no `#[serde(rename_all = "camelCase")]` on the
// Rust side, so the wire format uses `per_page`).  Renamed from `perPage`.
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
}

// === Auto-import (Stage F+) ===
// Mirrors src-tauri/src/services/auto_import.rs.  Returned by
// `get_last_auto_import` Tauri command as raw JSON string; frontend parses
// before showing a toast on App.tsx mount.
export interface ParseStats {
  files_scanned: number;
  lines_scanned: number;
  lines_matched: number;
  lines_parse_errored: number;
  sample_errors: string[];
}

export interface AgentImportEntry {
  agent_type: string;
  display_name: string;
  path: string;
  available: boolean;
  imported: number;
  skipped: number;
  errors: string[];
  parse_stats: ParseStats;
}

export interface AutoImportSummary {
  entries: AgentImportEntry[];
  total_imported: number;
  total_skipped: number;
  total_errors: number;
  started_at: number;
  finished_at: number;
}

// === PeriodsSummary (token-monitor-alignment Part A #1) ===
// Mirrors src-tauri/src/types.rs PeriodsSummary / PeriodsTriplet / PeriodWindowsPair / PeriodWindow

export interface PeriodWindow {
  key: string;
  ends_at: string;
}

export interface PeriodWindowsPair {
  today: PeriodWindow;
  month: PeriodWindow;
}

export interface PeriodsTriplet {
  today: UsageSummary;
  month: UsageSummary;
  all_time: UsageSummary;
}

export interface PeriodsSummary {
  periods: PeriodsTriplet;
  period_windows: PeriodWindowsPair;
  client_models: Record<string, Record<string, number>>;
  limits?: unknown | null;
}

// === Coding Plan Quota (Lane C) ===
//
// Mirrors src-tauri/src/types/subscription.rs SubscriptionQuota / QuotaTier /
// QuotaTierKind / CredentialStatus / TierStatus. Field names are SNAKE_CASE
// to match the Rust serde default output (see notes at the top of this file
// and the convention established by the token-usage types above).
//
// `SubscriptionQuota` is the wire shape returned by the
// `fetch_coding_plan_quota` IPC handler. Distinguishing features:
// - `success: false` always carries an `error` string; UI surfaces it
//   in the card footer.
// - `credential_status: "invalid"` ⇒ API key rejected (HTTP 401/403);
//   "expired" ⇒ upstream reported expiry; "unknown" ⇒ transport / parse
//   error; "valid" ⇒ upstream accepted.
// - `tiers` is empty on failure; on success it carries 1-3
//   QuotaTier entries (5-hour / weekly / monthly, in that order).
// - `queried_at_ms` is the local Unix epoch milliseconds when the
//   fetch completed — used by the frontend for cache math / "X min
//   ago" labels.
//
// QuotaTier.used and .limit are absolute values (USD / token / request
// count depending on provider). Volcengine AFP reports absolute
// `Used/Quota`; ZenMux reports absolute USD; most percentage-only
// providers (Kimi / GLM / MiniMax) leave both fields null.

export type CredentialStatus = "valid" | "invalid" | "expired" | "unknown";
export type TierStatus = "active" | "inactive" | "unknown";
export type QuotaTierKind = "five_hour" | "weekly" | "monthly";

export interface QuotaTier {
  kind: QuotaTierKind;
  label: string;
  used: number | null;
  limit: number | null;
  used_percent: number | null;
  remaining_percent: number | null;
  resets_at_ms: number | null;
  reset_description: string;
  status: TierStatus;
}

export interface SubscriptionQuota {
  provider_id: string;
  credential_status: CredentialStatus;
  credential_message: string | null;
  success: boolean;
  tiers: QuotaTier[];
  error: string | null;
  queried_at_ms: number;
}

export interface FetchCodingPlanQuotaRequest { id: number; }
export type FetchCodingPlanQuotaResponse = SubscriptionQuota;