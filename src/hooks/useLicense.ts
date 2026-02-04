import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export interface LicenseStatus {
  is_valid: boolean;
  license_key: string | null;
  activated_at: string | null;
  tier: "trial" | "licensed";
  documents_used: number | null;
  documents_limit: number | null;
  customer_name: string | null;
  customer_email: string | null;
}

export function useLicenseStatus() {
  return useQuery({
    queryKey: ["license-status"],
    queryFn: async () => {
      return await invoke<LicenseStatus>("get_license_status");
    },
  });
}

export function useActivateLicense() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (licenseKey: string) => {
      return await invoke<LicenseStatus>("activate_license", {
        licenseKey,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["license-status"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}

export function useDeactivateLicense() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      return await invoke("deactivate_license");
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["license-status"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}

export function useVerifyLicense() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      return await invoke<boolean>("verify_license");
    },
    onSuccess: (isValid) => {
      if (!isValid) {
        queryClient.invalidateQueries({ queryKey: ["license-status"] });
      }
    },
  });
}

// Debug only - activate test license
export function useActivateTestLicense() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      return await invoke<LicenseStatus>("activate_test_license");
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["license-status"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    },
  });
}
