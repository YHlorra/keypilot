




export type Visibility = 'visible' | 'masked';  
export type Theme = 'dark' | 'light' | 'auto';  


export interface ProviderField {
  id: number;
  provider_id: number;
  key: string;
  value: string;                       
  visibility: Visibility;
  sort_index: number;
  created_at: number;                  
  updated_at: number;
}

export interface Provider {
  id: number;
  name: string;
  preset: string | null;               
  is_preset: boolean;
  category_id: number;
  pinned: boolean;
  notes: string | null;
  icon: string | null;
  icon_color: string | null;
  sort_index: number;
  created_at: number;
  updated_at: number;
  fields: ProviderField[];             
}

export interface Category {
  id: number;
  name: string;
  is_default: boolean;
  sort_index: number;
  created_at: number;
  updated_at: number;
}







export interface QuotaSnapshot {
  total: number | null;
  used: number;
  remaining?: number;
  unit: 'USD' | 'CNY' | 'req' | 'GB' | 'token' | string;
  level?: 'green' | 'amber' | 'red' | 'ruby' | string;
  reset_at?: number;
}


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
  code: AppErrorCode;                  
  message: string;
}







export type ListProvidersResponse = Provider[];


export interface GetProviderRequest { id: number; }
export type GetProviderResponse = Provider;


export interface AddProviderRequest {
  name: string;
  preset: string | null;               
  category_id: number;
  pinned?: boolean;
  notes?: string;
  icon?: string;
  icon_color?: string;
  
  fields?: Array<Omit<ProviderField, 'id' | 'provider_id' | 'created_at' | 'updated_at'>>;
}
export type AddProviderResponse = Provider;



export interface UpdateProviderRequest {
  id: number;
  name?: string;
  category_id?: number;
  pinned?: boolean;
  notes?: string | null;
  icon?: string | null;
  icon_color?: string | null;
  
  fields?: Array<Omit<ProviderField, 'id' | 'provider_id' | 'created_at' | 'updated_at'>>;
}
export type UpdateProviderResponse = Provider;


export interface DeleteProviderRequest { id: number; }
export type DeleteProviderResponse = void;


export type ListCategoriesResponse = Category[];


export interface AddCategoryRequest { name: string; }
export type AddCategoryResponse = Category;


export interface DeleteCategoryRequest { id: number; migrate_to: number; }
export type DeleteCategoryResponse = void;


export interface FetchQuotaRequest { id: number; }
export type FetchQuotaResponse = QuotaSnapshot;


export type GetThemeResponse = Theme;


export interface SetThemeRequest { theme: Theme; }
export type SetThemeResponse = void;


export interface SetManualQuotaRequest { id: number; snapshot: QuotaSnapshot; }
export type SetManualQuotaResponse = void;










export interface TokenBreakdown {
  input?: number;
  output?: number;
  cache_read?: number;       
  cache_creation?: number;  
  reasoning?: number;       
}


export interface CostBreakdown {
  input?: number;
  output?: number;
  cache_read?: number;
  total?: number;
}


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


export interface UsageRecord extends UsageRecordInput {
  id: string;
}









export interface TokenUsageTickPayload {
  agent_type: string;
  imported: number;
  skipped: number;
  latest_at: number | null;
  total_today_tokens: number;
  total_today_cost_usd: number;
}


export interface UsageFilter {
  start_date?: string;
  end_date?: string;
  agent_type?: string;
  model?: string;
  provider?: string;
  status?: string;
}


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


export interface DailySeriesPoint {
  date: string;
  total_tokens: number;
  total_cost_usd: number;
  request_count: number;
}


export interface UsageSummary {
  total_tokens: number;
  total_cost_usd: number;
  total_requests: number;
  agent_pairs: AgentPair[];
  daily_series: DailySeriesPoint[];
}


export interface ImportResult {
  imported: number;
  skipped: number;
  errors: string[];
}


export type ImportFormat = 'jsonl' | 'csv';


export interface PricingEntry {
  model: string;
  input_cost_per_token: number;
  output_cost_per_token: number;
  cache_read_cost?: number;
  cache_creation_cost?: number;
  supports_reasoning?: boolean;
}





export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
}





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