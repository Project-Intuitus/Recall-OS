import { useMutation } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { RagResponse, ChunkWithScore, SearchRequest, SearchResult } from "../types";

export function useQuery() {
  return useMutation({
    mutationFn: ({
      query,
      conversationId,
    }: {
      query: string;
      conversationId?: string;
    }) => invoke<RagResponse>("query", { query, conversationId }),
  });
}

export function useQueryWithSources() {
  return useMutation({
    mutationFn: ({
      query,
      conversationId,
      maxChunks,
      documentIds,
    }: {
      query: string;
      conversationId?: string;
      maxChunks?: number;
      documentIds?: string[];
    }) =>
      invoke<RagResponse>("query_with_sources", {
        query,
        conversationId,
        maxChunks,
        documentIds,
      }),
  });
}

export function useSearch() {
  return useMutation({
    mutationFn: (request: SearchRequest) =>
      invoke<SearchResult>("search_documents", { request }),
  });
}

export function useHybridSearch() {
  return useMutation({
    mutationFn: ({ query, limit }: { query: string; limit?: number }) =>
      invoke<ChunkWithScore[]>("hybrid_search", { query, limit }),
  });
}
