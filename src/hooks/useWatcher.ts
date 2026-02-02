import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export interface WatcherStatus {
  is_running: boolean;
  watched_folders: string[];
  auto_ingest_enabled: boolean;
}

export function useWatcherStatus() {
  return useQuery({
    queryKey: ["watcher-status"],
    queryFn: () => invoke<WatcherStatus>("get_watcher_status"),
    refetchInterval: 5000, // Refresh every 5 seconds
  });
}

export function useStartWatcher() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke("start_watcher"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["watcher-status"] });
    },
  });
}

export function useStopWatcher() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke("stop_watcher"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["watcher-status"] });
    },
  });
}

export function useAddWatchedFolder() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (folderPath: string) => invoke("add_watched_folder", { folderPath }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["watcher-status"] });
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}

export function useRemoveWatchedFolder() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (folderPath: string) => invoke("remove_watched_folder", { folderPath }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["watcher-status"] });
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}

export function useToggleAutoIngest() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (enabled: boolean) => invoke("toggle_auto_ingest", { enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["watcher-status"] });
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}
