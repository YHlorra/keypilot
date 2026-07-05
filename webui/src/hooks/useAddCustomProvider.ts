import { useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import type { AddCustomProviderFormData } from '@/components/AddCustomProviderDialog';

export function useAddCustomProvider() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (data: AddCustomProviderFormData) => {
      const custom_spec = {
        protocol: data.protocol,
        base_url: data.base_url,
        auth_header: data.auth_header,
        notes: data.notes,
      };
      const fields = [
        { key: 'api_key', value: data.api_key, visibility: 'masked', sort_index: 1 },
      ];
      return invoke('add_provider', {
        req: {
          name: data.name,
          preset: null,
          category_id: 1,
          pinned: false,
          notes: data.notes,
          icon: null,
          icon_color: null,
          fields,
          custom_spec: JSON.stringify(custom_spec),
        },
      });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['providers'] });
    },
  });
}
