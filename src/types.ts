export interface Document {
  id: string;
  title: string;
  file_path: string;
  file_type: FileType;
  file_size: number;
  file_hash: string;
  mime_type: string | null;
  created_at: string;
  updated_at: string;
  ingested_at: string | null;
  status: DocumentStatus;
  error_message: string | null;
  metadata: Record<string, unknown>;
}

export type FileType = "pdf" | "text" | "markdown" | "video" | "audio" | "image" | "screenshot" | "unknown";

export type DocumentStatus = "pending" | "processing" | "completed" | "failed";

export interface Chunk {
  id: number;
  document_id: string;
  chunk_index: number;
  content: string;
  token_count: number;
  start_offset: number | null;
  end_offset: number | null;
  page_number: number | null;
  timestamp_start: number | null;
  timestamp_end: number | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface ChunkWithScore {
  chunk: Chunk;
  score: number;
  search_type: SearchType;
}

export type SearchType = "vector" | "fts" | "hybrid";

export interface Citation {
  chunk_id: number;
  document_id: string;
  document_title: string;
  content_snippet: string;
  page_number: number | null;
  timestamp: number | null;
  relevance_score: number;
}

export interface SourceChunk {
  chunk_id: number;
  document_id: string;
  document_title: string;
  content: string;
  page_number: number | null;
  timestamp: number | null;
  relevance_score: number;
  search_type: SearchType;
}

export interface RagResponse {
  answer: string;
  citations: Citation[];
  sources: SourceChunk[];
  conversation_id: string;
}

export interface Message {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  citations: Citation[];
  created_at: string;
}

export interface Conversation {
  id: string;
  title: string | null;
  created_at: string;
  updated_at: string;
}

export interface Settings {
  gemini_api_key: string | null;
  embedding_model: string;
  ingestion_model: string;
  reasoning_model: string;
  chunk_size: number;
  chunk_overlap: number;
  max_context_chunks: number;
  video_segment_duration: number;
  keyframe_interval: number;
  watched_folders: string[];
  auto_ingest_enabled: boolean;
  // Screen capture settings
  screen_capture_enabled: boolean;
  capture_interval_secs: number;
  capture_mode: "full_screen" | "active_window";
  capture_app_filter: "none" | "whitelist" | "blacklist";
  capture_app_list: string[];
  capture_retention_days: number;
  capture_hotkey: string;
  // License settings
  license_key: string | null;
  license_activated_at: string | null;
}

export interface LicenseStatus {
  is_valid: boolean;
  license_key: string | null;
  activated_at: string | null;
  tier: "trial" | "licensed";
  documents_used: number | null;
  documents_limit: number | null;
}

export interface CaptureStatus {
  enabled: boolean;
  scheduler_running: boolean;
  paused: boolean;
  mode: string;
  interval_secs: number;
  capture_count: number;
  last_capture: string | null;
  hotkey: string;
}

export interface AppInfo {
  process_name: string;
  window_title: string;
  is_foreground: boolean;
}

export interface IngestionStats {
  total_documents: number;
  completed_documents: number;
  failed_documents: number;
  pending_documents: number;
  processing_documents: number;
  total_chunks: number;
  total_size_bytes: number;
}

export interface IngestionProgress {
  document_id: string;
  file_path: string;
  stage: IngestionStage;
  progress: number;
  message: string;
}

export type IngestionStage =
  | "queued"
  | "extracting"
  | "chunking"
  | "embedding"
  | "indexing"
  | "completed"
  | "failed";

export interface SearchRequest {
  query: string;
  limit?: number;
  document_ids?: string[];
}

export interface SearchResult {
  chunks: ChunkWithScore[];
  total: number;
}
