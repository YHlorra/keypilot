



import { invoke } from "@tauri-apps/api/core";
import type {
  ListProvidersResponse, GetProviderRequest, GetProviderResponse,
  AddProviderRequest, AddProviderResponse, UpdateProviderRequest, UpdateProviderResponse,
  DeleteProviderRequest, DeleteProviderResponse, ListCategoriesResponse,
  AddCategoryRequest, AddCategoryResponse, DeleteCategoryRequest, DeleteCategoryResponse,
  FetchQuotaRequest, FetchQuotaResponse,
  GetThemeResponse, SetThemeRequest, SetThemeResponse,
  SetManualQuotaRequest, SetManualQuotaResponse,
  UsageFilter,
  ImportFormat, ImportResult, PricingEntry,
  QuotaSnapshot,
  PeriodsSummary,
  FetchCodingPlanQuotaRequest, FetchCodingPlanQuotaResponse,
} from "@/types/api";
import { executeAction } from "@/lib/action-registry";



export async function listProviders(): Promise<ListProvidersResponse> {
  return invoke<ListProvidersResponse>("list_providers");
}

export async function getProvider(req: GetProviderRequest): Promise<GetProviderResponse> {
  return invoke<GetProviderResponse>("get_provider", { id: req.id });
}

export async function addProvider(req: AddProviderRequest): Promise<AddProviderResponse> {
  return invoke<AddProviderResponse>("add_provider", { req });
}

export async function updateProvider(req: UpdateProviderRequest): Promise<UpdateProviderResponse> {
  return invoke<UpdateProviderResponse>("update_provider", { req });
}

export async function deleteProvider(req: DeleteProviderRequest): Promise<DeleteProviderResponse> {
  return invoke<DeleteProviderResponse>("delete_provider", { id: req.id });
}

export async function listCategories(): Promise<ListCategoriesResponse> {
  return invoke<ListCategoriesResponse>("list_categories");
}

export async function addCategory(req: AddCategoryRequest): Promise<AddCategoryResponse> {
  return invoke<AddCategoryResponse>("add_category", { req });
}

export async function deleteCategory(req: DeleteCategoryRequest): Promise<DeleteCategoryResponse> {
  return invoke<DeleteCategoryResponse>("delete_category", { req });
}

export async function fetchQuota(req: FetchQuotaRequest): Promise<FetchQuotaResponse> {
  return invoke<FetchQuotaResponse>("fetch_quota", { id: req.id });
}






export async function fetchCodingPlanQuota(req: FetchCodingPlanQuotaRequest): Promise<FetchCodingPlanQuotaResponse> {
  return invoke<FetchCodingPlanQuotaResponse>("fetch_coding_plan_quota", { id: req.id });
}

export async function getTheme(): Promise<GetThemeResponse> {
  return invoke<GetThemeResponse>("get_theme");
}

export async function setTheme(req: SetThemeRequest): Promise<SetThemeResponse> {
  return invoke<SetThemeResponse>("set_theme", { theme: req.theme });
}

export async function setManualQuota(req: SetManualQuotaRequest): Promise<SetManualQuotaResponse> {
  return invoke<SetManualQuotaResponse>("set_manual_quota", { req });
}





export async function getUsagePeriodsSummary(filter: UsageFilter): Promise<PeriodsSummary> {
  return invoke<PeriodsSummary>("get_usage_periods_summary", { filter });
}

export async function importUsage(
  content: string,
  format: ImportFormat,
  sourceHint?: string
): Promise<ImportResult> {
  return invoke<ImportResult>("import_usage", { content, format, source_hint: sourceHint });
}

export async function getPricing(): Promise<PricingEntry[]> {
  return invoke<PricingEntry[]>("get_pricing", {});
}



export async function getLastAutoImport(): Promise<string | null> {
  return invoke<string | null>("get_last_auto_import", {});
}



export interface CopyCredentialResponse {
  value: string;
  field_key: string;
}
export async function copyCredential(req: { id: number; field_key?: string }): Promise<CopyCredentialResponse> {
  return executeAction("provider.copy_credential", req) as Promise<CopyCredentialResponse>;
}

export interface TestAndRefreshResponse {
  test: string;
  quota: QuotaSnapshot | null;
}
export async function testAndRefresh(req: { id: number }): Promise<TestAndRefreshResponse> {
  return executeAction("provider.test_and_refresh", req) as Promise<TestAndRefreshResponse>;
}

export async function openForEdit(req: { id: number }): Promise<GetProviderResponse> {
  return executeAction("provider.open_for_edit", req) as Promise<GetProviderResponse>;
}

export async function deleteProviderViaAction(req: { id: number }): Promise<void> {
  return executeAction("provider.delete", req) as Promise<void>;
}


