import { useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import type { AddCustomProviderFormData } from '@/components/AddCustomProviderDialog';

export interface PreflightInput {
  form: AddCustomProviderFormData;
}

export interface PreflightResult {
  ok: boolean;
  message: string;
}

export function usePreflight() {
  return useMutation<PreflightResult, Error, PreflightInput>({
    mutationFn: async ({ form }) => {
      const custom_spec = {
        protocol: form.protocol,
        base_url: form.base_url,
        auth_header: form.auth_header,
        notes: form.notes,
      };
      return invoke<PreflightResult>('preflight', {
        req: {
          custom_spec,
          api_key: form.api_key,
        },
      });
    },
  });
}
