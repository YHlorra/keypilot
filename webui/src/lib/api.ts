// webui/src/lib/api.ts (Phase 2 Lane C — real IPC wiring)
// All 12 IPC functions now use @tauri-apps/api/core::invoke (replaces Phase 2 Lane B1 mocks).
// @see openspec/changes/v0.1-general-credentials/design.md §7 for Rust command signatures.

import { invoke } from "@tauri-apps/api/core";
import type {
  ListProvidersResponse, GetProviderRequest, GetProviderResponse,
  AddProviderRequest, AddProviderResponse, UpdateProviderRequest, UpdateProviderResponse,
  DeleteProviderRequest, DeleteProviderResponse, ListCategoriesResponse,
  AddCategoryRequest, AddCategoryResponse, DeleteCategoryRequest, DeleteCategoryResponse,
  TestConnectionRequest, TestConnectionResponse, FetchQuotaRequest, FetchQuotaResponse,
  GetThemeResponse, SetThemeRequest, SetThemeResponse,
  PinProviderResponse,
  UnpinProviderResponse,
  QuitAppResponse,
  SetManualQuotaRequest, SetManualQuotaResponse,
} from "@/types/api";

// 12 IPC functions — real Tauri invoke wiring

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

export async function testConnection(req: TestConnectionRequest): Promise<TestConnectionResponse> {
  return invoke<TestConnectionResponse>("test_connection", { id: req.id });
}

export async function fetchQuota(req: FetchQuotaRequest): Promise<FetchQuotaResponse> {
  return invoke<FetchQuotaResponse>("fetch_quota", { id: req.id });
}

export async function getTheme(): Promise<GetThemeResponse> {
  return invoke<GetThemeResponse>("get_theme");
}

export async function setTheme(req: SetThemeRequest): Promise<SetThemeResponse> {
  return invoke<SetThemeResponse>("set_theme", { theme: req.theme });
}

export async function pinProvider(id: number): Promise<PinProviderResponse> {
  return invoke<PinProviderResponse>("pin_provider", { provider_id: id });
}

export async function unpinProvider(id: number): Promise<UnpinProviderResponse> {
  return invoke<UnpinProviderResponse>("unpin_provider", { provider_id: id });
}

export async function quitApp(): Promise<QuitAppResponse> {
  return invoke<QuitAppResponse>("quit_app");
}

export async function setManualQuota(req: SetManualQuotaRequest): Promise<SetManualQuotaResponse> {
  // TODO V0.1.1: switch to real IPC once backend adds set_manual_quota command
  // For V0.1: localStorage-only (manual quota never hits backend)
  try {
    const key = `keypilot_manual_quota_${req.id}`;
    localStorage.setItem(key, JSON.stringify(req.snapshot));
  } catch {
    // Silently ignore storage errors
  }
  return Promise.resolve();
}
