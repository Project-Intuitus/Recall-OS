import { useState, useEffect } from "react";
import { X, FileText, Film, Music, Image, Camera, ExternalLink, ChevronLeft, ChevronRight, Eye, List, BookOpen } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useDocument, useDocumentChunks } from "../hooks/useDocuments";
import type { SourceChunk } from "../types";
import PdfViewer from "./PdfViewer";
import VideoPlayer from "./VideoPlayer";
import AudioPlayer from "./AudioPlayer";
import clsx from "clsx";

interface SourcePanelProps {
  source: SourceChunk;
  onClose: () => void;
}

type ViewMode = "preview" | "chunks";

interface PageSection {
  pageNumber: number | null;
  content: string;
}

// Clean up OCR text within a section
function cleanOcrText(text: string): string {
  return text
    // Remove common OCR noise patterns (random caps, symbols)
    .replace(/[|\\\/]{2,}/g, " ")
    // Clean up excessive punctuation
    .replace(/[.]{3,}/g, "...")
    // Remove standalone single characters that are likely noise
    .replace(/\s[A-Z]\s(?=[A-Z]\s)/g, " ")
    // Normalize quotes
    .replace(/[""]/g, '"')
    .replace(/['']/g, "'")
    // Clean up whitespace
    .replace(/\s{2,}/g, " ")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

// Parse content into page sections
function parsePageSections(content: string): PageSection[] {
  const sections: PageSection[] = [];
  const pagePattern = /---\s*Page\s*(\d+)\s*---/gi;

  let match;

  // Find all page markers
  const matches: { index: number; pageNum: number; length: number }[] = [];
  while ((match = pagePattern.exec(content)) !== null) {
    matches.push({
      index: match.index,
      pageNum: parseInt(match[1], 10),
      length: match[0].length
    });
  }

  if (matches.length === 0) {
    // No page markers, return as single section
    const cleaned = cleanOcrText(content);
    if (cleaned) {
      sections.push({ pageNumber: null, content: cleaned });
    }
    return sections;
  }

  // Content before first page marker
  if (matches[0].index > 0) {
    const beforeContent = cleanOcrText(content.slice(0, matches[0].index));
    if (beforeContent) {
      sections.push({ pageNumber: null, content: beforeContent });
    }
  }

  // Process each page section
  for (let i = 0; i < matches.length; i++) {
    const current = matches[i];
    const next = matches[i + 1];

    const startIndex = current.index + current.length;
    const endIndex = next ? next.index : content.length;

    const pageContent = cleanOcrText(content.slice(startIndex, endIndex));
    if (pageContent) {
      sections.push({ pageNumber: current.pageNum, content: pageContent });
    }
  }

  return sections;
}

// Render formatted chunk content with page sections
function FormattedChunkContent({ content }: { content: string }) {
  const sections = parsePageSections(content);

  if (sections.length === 0) {
    return (
      <p className="text-slate-400 italic">No content available</p>
    );
  }

  return (
    <div className="space-y-4">
      {sections.map((section, index) => (
        <div key={index} className="relative">
          {section.pageNumber !== null && (
            <div className="flex items-center gap-2 mb-2 pb-2 border-b border-slate-600/50">
              <BookOpen className="w-4 h-4 text-blue-400" />
              <span className="text-sm font-medium text-blue-400">
                Page {section.pageNumber}
              </span>
            </div>
          )}
          <p className="text-slate-200 leading-relaxed text-sm whitespace-pre-wrap">
            {section.content}
          </p>
        </div>
      ))}
    </div>
  );
}

// Get a clean preview of chunk content
function getChunkPreview(content: string): string {
  // Remove page markers for preview
  let preview = content
    .replace(/---\s*Page\s*\d+\s*---/gi, " ")
    .replace(/\s+/g, " ")
    .trim();

  // Get first meaningful sentence or fragment
  const firstSentence = preview.match(/^[^.!?]+[.!?]/);
  if (firstSentence && firstSentence[0].length > 30) {
    return firstSentence[0];
  }

  // Otherwise return first 120 chars
  return preview.slice(0, 120) + (preview.length > 120 ? "..." : "");
}

export default function SourcePanel({ source, onClose }: SourcePanelProps) {
  const { data: document, isError: isDocError, error: docError } = useDocument(source.document_id);
  const { data: chunks, isError: isChunksError, error: chunksError } = useDocumentChunks(source.document_id);
  const [currentChunkIndex, setCurrentChunkIndex] = useState(0);
  const [viewMode, setViewMode] = useState<ViewMode>("preview");

  // Handle error states
  if (isDocError || isChunksError) {
    const errorMessage = docError instanceof Error ? docError.message :
                        chunksError instanceof Error ? chunksError.message :
                        "Failed to load source data";
    return (
      <div className="flex flex-col h-full bg-slate-800">
        <div className="flex items-center justify-between p-4 border-b border-slate-700">
          <h3 className="font-medium text-red-400">Error Loading Source</h3>
          <button
            onClick={onClose}
            className="p-2 hover:bg-slate-700 rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
        <div className="flex-1 flex items-center justify-center p-8">
          <div className="text-center">
            <div className="text-red-400 mb-2">Failed to load source</div>
            <div className="text-sm text-slate-500">{errorMessage}</div>
          </div>
        </div>
      </div>
    );
  }

  // Find the chunk in the list
  useEffect(() => {
    if (chunks) {
      const index = chunks.findIndex((c) => c.id === source.chunk_id);
      if (index >= 0) {
        setCurrentChunkIndex(index);
      }
    }
  }, [chunks, source.chunk_id]);

  const currentChunk = chunks?.[currentChunkIndex];
  const isPdf = document?.file_type === "pdf";
  const isVideo = document?.file_type === "video";
  const isAudio = document?.file_type === "audio";
  const isImage = document?.file_type === "image";
  const isScreenshot = document?.file_type === "screenshot";
  const isVisualMedia = isImage || isScreenshot;

  const formatTimestamp = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  };

  const getFileIcon = () => {
    if (!document) return <FileText className="w-5 h-5" />;

    switch (document.file_type) {
      case "video":
        return <Film className="w-5 h-5" />;
      case "audio":
        return <Music className="w-5 h-5" />;
      case "image":
        return <Image className="w-5 h-5" />;
      case "screenshot":
        return <Camera className="w-5 h-5" />;
      default:
        return <FileText className="w-5 h-5" />;
    }
  };

  const handleOpenFile = async () => {
    if (document?.file_path) {
      try {
        // Use Tauri shell command to open file in default app
        await invoke("open_file_in_default_app", { path: document.file_path });
      } catch (err) {
        console.error("Failed to open file:", err);
      }
    }
  };

  return (
    <div className="flex flex-col h-full bg-slate-800">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-slate-700">
        <div className="flex items-center gap-3 min-w-0">
          <span className="text-blue-400">{getFileIcon()}</span>
          <div className="min-w-0">
            <h3 className="font-medium truncate">{source.document_title}</h3>
            {currentChunk && (
              <div className="flex items-center gap-2 text-sm text-slate-400">
                {currentChunk.page_number && (
                  <span>Page {currentChunk.page_number}</span>
                )}
                {currentChunk.timestamp_start !== null && (
                  <span>
                    {formatTimestamp(currentChunk.timestamp_start)}
                    {currentChunk.timestamp_end !== null && (
                      <> - {formatTimestamp(currentChunk.timestamp_end)}</>
                    )}
                  </span>
                )}
              </div>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          {/* View mode toggle for media files */}
          {(isPdf || isVideo || isAudio || isVisualMedia) && (
            <div className="flex items-center bg-slate-700 rounded-lg p-0.5">
              <button
                onClick={() => setViewMode("preview")}
                className={clsx(
                  "p-1.5 rounded-md transition-colors",
                  viewMode === "preview"
                    ? "bg-blue-600 text-white"
                    : "text-slate-400 hover:text-white"
                )}
                title="PDF Preview"
              >
                <Eye className="w-4 h-4" />
              </button>
              <button
                onClick={() => setViewMode("chunks")}
                className={clsx(
                  "p-1.5 rounded-md transition-colors",
                  viewMode === "chunks"
                    ? "bg-blue-600 text-white"
                    : "text-slate-400 hover:text-white"
                )}
                title="Chunk List"
              >
                <List className="w-4 h-4" />
              </button>
            </div>
          )}
          <button
            onClick={onClose}
            className="p-2 hover:bg-slate-700 rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        {/* PDF Preview Mode */}
        {isPdf && viewMode === "preview" && document && (
          <PdfViewer
            filePath={document.file_path}
            pageNumber={currentChunk?.page_number}
            highlightText={source.content}
          />
        )}

        {/* Video Preview Mode */}
        {isVideo && viewMode === "preview" && document && (
          <VideoPlayer
            filePath={document.file_path}
            currentTime={currentChunk?.timestamp_start ?? undefined}
          />
        )}

        {/* Audio Preview Mode */}
        {isAudio && viewMode === "preview" && document && (
          <AudioPlayer
            filePath={document.file_path}
            currentTime={currentChunk?.timestamp_start ?? undefined}
            title={document.title}
          />
        )}

        {/* Image/Screenshot Preview Mode */}
        {isVisualMedia && viewMode === "preview" && document && (
          <div className="h-full flex flex-col overflow-hidden">
            <div className="flex-1 overflow-auto p-4 flex items-center justify-center bg-slate-900/50">
              <img
                src={convertFileSrc(document.file_path)}
                alt={document.title}
                className="max-w-full max-h-full object-contain rounded-lg shadow-lg"
                onError={(e) => {
                  console.error("Failed to load image:", document.file_path);
                  (e.target as HTMLImageElement).style.display = "none";
                }}
              />
            </div>
            {/* Screenshot metadata */}
            {isScreenshot && document.metadata && (() => {
              const meta = document.metadata as Record<string, string>;
              return (
                <div className="p-3 bg-slate-800 border-t border-slate-700 text-sm">
                  <div className="flex flex-wrap gap-4 text-slate-400">
                    {meta.source_app && (
                      <span>
                        <span className="text-slate-500">App:</span> {meta.source_app}
                      </span>
                    )}
                    {meta.window_title && (
                      <span className="truncate max-w-[300px]">
                        <span className="text-slate-500">Window:</span> {meta.window_title}
                      </span>
                    )}
                    {meta.resolution && (
                      <span>
                        <span className="text-slate-500">Size:</span> {meta.resolution}
                      </span>
                    )}
                  </div>
                </div>
              );
            })()}
          </div>
        )}

        {/* Chunks/Text Mode (default for non-media or when toggled) */}
        {((!isPdf && !isVideo && !isAudio && !isVisualMedia) || viewMode === "chunks") && (
          <div className="h-full overflow-y-auto p-4">
            {/* Current chunk display */}
            <div className="bg-gradient-to-br from-slate-700/50 to-slate-800/50 border border-slate-600/50 rounded-xl p-5 mb-4">
              {/* Header */}
              <div className="flex items-center justify-between mb-4 pb-3 border-b border-slate-600/50">
                <div className="flex items-center gap-3">
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-blue-500"></div>
                    <span className="text-sm font-medium text-slate-200">
                      {currentChunk?.page_number ? `Page ${currentChunk.page_number}` : "Matched Content"}
                    </span>
                  </div>
                  {currentChunk?.timestamp_start !== null && currentChunk?.timestamp_start !== undefined && (
                    <span className="text-xs text-slate-400 bg-slate-600/50 px-2 py-0.5 rounded">
                      {formatTimestamp(currentChunk.timestamp_start)}
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-slate-400">
                    {(source.relevance_score * 100).toFixed(0)}% match
                  </span>
                  <span className="text-xs px-2 py-0.5 bg-blue-600/30 text-blue-300 rounded-full border border-blue-500/30">
                    {source.search_type}
                  </span>
                </div>
              </div>

              {/* Content - structured page sections */}
              <div className="prose prose-invert prose-sm max-w-none">
                <FormattedChunkContent content={currentChunk?.content || source.content} />
              </div>
            </div>

            {/* Video/Audio timestamp info */}
            {(isVideo || isAudio) && currentChunk && currentChunk.timestamp_start !== null && (
              <div className="mb-4 p-4 bg-gradient-to-r from-purple-900/20 to-slate-800/20 border border-purple-500/30 rounded-lg">
                <div className="flex items-center gap-3">
                  {isVideo ? <Film className="w-5 h-5 text-purple-400" /> : <Music className="w-5 h-5 text-purple-400" />}
                  <div>
                    <p className="text-sm text-slate-200 font-medium">
                      Timestamp: {formatTimestamp(currentChunk.timestamp_start)}
                      {currentChunk.timestamp_end !== null && ` - ${formatTimestamp(currentChunk.timestamp_end)}`}
                    </p>
                    <p className="text-xs text-slate-400 mt-0.5">
                      Click "Open" below to play in your default {isVideo ? "video" : "audio"} player
                    </p>
                  </div>
                </div>
              </div>
            )}

            {/* Image/Screenshot info */}
            {isVisualMedia && (
              <div className="mb-4 p-4 bg-gradient-to-r from-green-900/20 to-slate-800/20 border border-green-500/30 rounded-lg">
                <div className="flex items-center gap-3">
                  {isScreenshot ? (
                    <Camera className="w-5 h-5 text-green-400" />
                  ) : (
                    <Image className="w-5 h-5 text-green-400" />
                  )}
                  <p className="text-sm text-slate-300">
                    {isScreenshot
                      ? "OCR-extracted text from screenshot"
                      : "AI-generated description of image content"
                    }
                  </p>
                </div>
              </div>
            )}

            {/* Chunk navigator */}
            {chunks && chunks.length > 0 && (
              <div className="mt-6">
                {/* Header with navigation */}
                <div className="flex items-center justify-between mb-4">
                  <h4 className="text-sm font-semibold text-slate-200 flex items-center gap-2">
                    <List className="w-4 h-4 text-slate-400" />
                    All Chunks
                    <span className="text-xs font-normal text-slate-500 bg-slate-700 px-2 py-0.5 rounded-full">
                      {chunks.length}
                    </span>
                  </h4>
                  <div className="flex items-center gap-1 bg-slate-700/50 rounded-lg p-1">
                    <button
                      onClick={() => setCurrentChunkIndex(Math.max(0, currentChunkIndex - 1))}
                      disabled={currentChunkIndex === 0}
                      className="p-1.5 hover:bg-slate-600 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                    >
                      <ChevronLeft className="w-4 h-4" />
                    </button>
                    <span className="text-sm text-slate-300 min-w-[60px] text-center font-medium">
                      {currentChunkIndex + 1} / {chunks.length}
                    </span>
                    <button
                      onClick={() => setCurrentChunkIndex(Math.min(chunks.length - 1, currentChunkIndex + 1))}
                      disabled={currentChunkIndex === chunks.length - 1}
                      className="p-1.5 hover:bg-slate-600 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                    >
                      <ChevronRight className="w-4 h-4" />
                    </button>
                  </div>
                </div>

                {/* Chunk list */}
                <div className="space-y-2 max-h-[280px] overflow-y-auto pr-1">
                  {chunks.map((chunk, index) => (
                    <button
                      key={chunk.id}
                      onClick={() => setCurrentChunkIndex(index)}
                      className={clsx(
                        "w-full text-left p-3 rounded-lg transition-all duration-150",
                        index === currentChunkIndex
                          ? "bg-blue-600/20 border-2 border-blue-500/50 shadow-lg shadow-blue-500/10"
                          : "bg-slate-700/30 border border-slate-600/30 hover:bg-slate-700/50 hover:border-slate-500/50"
                      )}
                    >
                      <div className="flex items-center justify-between mb-2">
                        <div className="flex items-center gap-2">
                          <span className={clsx(
                            "w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium",
                            index === currentChunkIndex
                              ? "bg-blue-500 text-white"
                              : "bg-slate-600 text-slate-300"
                          )}>
                            {chunk.chunk_index + 1}
                          </span>
                          {chunk.page_number && (
                            <span className="text-xs text-slate-400">
                              Page {chunk.page_number}
                            </span>
                          )}
                          {chunk.timestamp_start !== null && (
                            <span className="text-xs text-slate-400">
                              {formatTimestamp(chunk.timestamp_start)}
                            </span>
                          )}
                        </div>
                        <span className="text-xs text-slate-500 bg-slate-600/50 px-2 py-0.5 rounded">
                          {chunk.token_count} tokens
                        </span>
                      </div>
                      <p className="text-sm text-slate-300 line-clamp-2 leading-relaxed">
                        {getChunkPreview(chunk.content)}
                      </p>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      {document && (
        <div className="p-4 border-t border-slate-700">
          <div className="flex items-center justify-between text-sm">
            <span className="text-slate-400 truncate max-w-[70%]">
              {document.file_path}
            </span>
            <button
              onClick={handleOpenFile}
              className="flex items-center gap-1 text-blue-400 hover:text-blue-300"
            >
              <ExternalLink className="w-4 h-4" />
              Open
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
