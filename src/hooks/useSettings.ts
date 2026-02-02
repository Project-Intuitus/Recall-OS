import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "../types";

export function useSettings() {
  return useQuery({
    queryKey: ["settings"],
    queryFn: () => invoke<Settings>("get_settings"),
  });
}

export function useUpdateSettings() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (settings: Settings) => invoke("update_settings", { newSettings: settings }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}

export function useValidateApiKey() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (apiKey: string) => invoke<boolean>("validate_api_key", { apiKey }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}

export function useGetApiKeyUnmasked() {
  return useMutation({
    mutationFn: () => invoke<string | null>("get_api_key_unmasked"),
  });
}

export function useClearApiKey() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke("clear_api_key"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}
