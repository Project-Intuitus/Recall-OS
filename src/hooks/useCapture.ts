import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { CaptureStatus, AppInfo } from "../types";

/**
 * Hook to get the current capture status
 */
export function useCaptureStatus() {
  return useQuery({
    queryKey: ["captureStatus"],
    queryFn: () => invoke<CaptureStatus>("get_capture_status"),
    refetchInterval: 5000, // Refresh every 5 seconds
  });
}

/**
 * Hook to start screen capture
 */
export function useStartCapture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke("start_screen_capture"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["captureStatus"] });
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}

/**
 * Hook to stop screen capture
 */
export function useStopCapture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke("stop_screen_capture"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["captureStatus"] });
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}

/**
 * Hook to capture a screenshot now
 */
export function useCaptureNow() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke<string>("capture_now"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["captureStatus"] });
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
  });
}

/**
 * Hook to pause screen capture
 */
export function usePauseCapture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke("pause_screen_capture"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["captureStatus"] });
    },
  });
}

/**
 * Hook to resume screen capture
 */
export function useResumeCapture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke("resume_screen_capture"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["captureStatus"] });
    },
  });
}

/**
 * Hook to get list of running applications
 */
export function useRunningApps() {
  return useQuery({
    queryKey: ["runningApps"],
    queryFn: () => invoke<AppInfo[]>("get_running_applications"),
    staleTime: 10000, // Cache for 10 seconds
  });
}

/**
 * Hook to update capture settings
 */
export function useUpdateCaptureSettings() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (settings: {
      enabled: boolean;
      interval_secs: number;
      mode: string;
      filter_mode: string;
      app_list: string[];
      retention_days: number;
      hotkey: string;
    }) =>
      invoke("update_capture_settings", {
        enabled: settings.enabled,
        intervalSecs: settings.interval_secs,
        mode: settings.mode,
        filterMode: settings.filter_mode,
        appList: settings.app_list,
        retentionDays: settings.retention_days,
        hotkey: settings.hotkey,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["captureStatus"] });
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}

/**
 * Hook to clean up old captures
 */
export function useCleanupCaptures() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke<number>("cleanup_old_captures"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
  });
}
