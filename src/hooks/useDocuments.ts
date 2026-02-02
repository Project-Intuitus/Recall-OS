import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { Document, Chunk, IngestionStats } from "../types";

export function useDocuments() {
  return useQuery({
    queryKey: ["documents"],
    queryFn: () => invoke<Document[]>("get_documents"),
    // Reduced from 5s to 30s - auto-ingest events update UI via Tauri events
    refetchInterval: 30000,
    // Only refetch when window is focused
    refetchOnWindowFocus: true,
  });
}

export function useDocument(id: string | null) {
  return useQuery({
    queryKey: ["document", id],
    queryFn: () => invoke<Document | null>("get_document", { id }),
    enabled: !!id,
  });
}

export function useDocumentChunks(documentId: string | null) {
  return useQuery({
    queryKey: ["chunks", documentId],
    queryFn: () => invoke<Chunk[]>("get_chunks_for_document", { documentId }),
    enabled: !!documentId,
  });
}

export function useDeleteDocument() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => invoke("delete_document", { id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    },
    onError: (error) => {
      console.error("Failed to delete document:", error);
    },
  });
}

export function useIngestionStats() {
  return useQuery({
    queryKey: ["stats"],
    queryFn: () => invoke<IngestionStats>("get_ingestion_stats"),
    // Reduced from 5s to 30s for less aggressive polling
    refetchInterval: 30000,
    refetchOnWindowFocus: true,
  });
}

export function useIngestFile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (path: string) => invoke<Document>("ingest_file", { path }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    },
    onError: (error) => {
      console.error("Failed to ingest file:", error);
    },
  });
}

export function useIngestDirectory() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ path, recursive }: { path: string; recursive?: boolean }) =>
      invoke<Document[]>("ingest_directory", { path, recursive }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    },
    onError: (error) => {
      console.error("Failed to ingest directory:", error);
    },
  });
}

export function useReingestDocument() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => invoke<Document>("reingest_document", { id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    },
    onError: (error) => {
      console.error("Failed to reingest document:", error);
    },
  });
}

export function useResetDatabase() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke("reset_database"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
      queryClient.invalidateQueries({ queryKey: ["chunks"] });
      queryClient.invalidateQueries({ queryKey: ["watcher-status"] });
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
    onError: (error) => {
      console.error("Failed to reset database:", error);
    },
  });
}

export interface ContentCategory {
  category: string;
  confidence: number;
}

export function useContentCategories() {
  return useQuery({
    queryKey: ["contentCategories"],
    queryFn: () => invoke<string[]>("get_content_categories"),
  });
}

export function useCategorizeDocument() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (documentId: string) =>
      invoke<ContentCategory>("categorize_document", { documentId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
    onError: (error) => {
      console.error("Failed to categorize document:", error);
    },
  });
}

export function useCategorizeAllDocuments() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke<[string, string][]>("categorize_all_documents"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["documents"] });
    },
    onError: (error) => {
      console.error("Failed to categorize documents:", error);
    },
  });
}
