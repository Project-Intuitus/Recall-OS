import { useState, useEffect, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  FolderPlus,
  FilePlus,
  Settings,
  Database,
  FileText,
  Film,
  Music,
  Image,
  Camera,
  Loader2,
  CheckCircle,
  XCircle,
  ChevronRight,
  ChevronDown,
  Trash2,
  RefreshCw,
  HelpCircle,
  ArrowUpDown,
  Clock,
  SortAsc,
  FileType as FileTypeIcon,
  HardDrive,
  MessageSquarePlus,
  MessageSquare,
  Check,
  Filter,
  List,
  LayoutGrid,
  Sparkles,
  FolderOpen,
  Shield,
} from "lucide-react";

// Custom icon component matching the app icon
const RecallIcon = ({ className }: { className?: string }) => (
  <svg viewBox="0 0 24 24" fill="none" className={className}>
    <circle cx="12" cy="12" r="9.5" stroke="currentColor" strokeWidth="2"/>
    <circle cx="12" cy="12" r="3.5" fill="currentColor"/>
  </svg>
);
import { useDocuments, useIngestFile, useIngestDirectory, useDeleteDocument, useReingestDocument, useIngestionStats, useCategorizeAllDocuments } from "../hooks/useDocuments";
import { useConversations, useDeleteConversation } from "../hooks/useConversations";
import type { Document, IngestionProgress, FileType, Conversation } from "../types";
import clsx from "clsx";

interface SidebarProps {
  onSettingsClick: () => void;
  onHelpClick: () => void;
  onLicenseClick: () => void;
  ingestionProgress: IngestionProgress[];
  onDocumentClick?: (document: Document) => void;
  highlightedDocIds?: string[];
  currentConversationId: string | null;
  onConversationSelect: (id: string | null) => void;
  onNewConversation: () => void;
  selectedDocumentIds: string[];
  onDocumentSelectionChange: (ids: string[]) => void;
  isLicensed?: boolean;
  trialDocsUsed?: number;
  trialDocsLimit?: number;
}

type SortOption = "recent" | "alphabetical" | "type" | "size";
type ViewMode = "flat" | "grouped" | "content";

const sortOptions: { value: SortOption; label: string; icon: React.ReactNode }[] = [
  { value: "recent", label: "Recent", icon: <Clock className="w-3.5 h-3.5" /> },
  { value: "alphabetical", label: "A-Z", icon: <SortAsc className="w-3.5 h-3.5" /> },
  { value: "type", label: "Type", icon: <FileTypeIcon className="w-3.5 h-3.5" /> },
  { value: "size", label: "Size", icon: <HardDrive className="w-3.5 h-3.5" /> },
];

const categoryOrder: FileType[] = ["pdf", "text", "markdown", "video", "audio", "image", "screenshot", "unknown"];

const categoryLabels: Record<FileType, string> = {
  pdf: "PDFs",
  text: "Text Files",
  markdown: "Markdown",
  video: "Videos",
  audio: "Audio",
  image: "Images",
  screenshot: "Screenshots",
  unknown: "Other",
};

export default function Sidebar({
  onSettingsClick,
  onHelpClick,
  onLicenseClick,
  ingestionProgress,
  onDocumentClick,
  highlightedDocIds = [],
  currentConversationId,
  onConversationSelect,
  onNewConversation,
  selectedDocumentIds,
  onDocumentSelectionChange,
  isLicensed = false,
  trialDocsUsed,
  trialDocsLimit,
}: SidebarProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [sortBy, setSortBy] = useState<SortOption>("recent");
  const [showSortMenu, setShowSortMenu] = useState(false);
  const [filterMode, setFilterMode] = useState<"all" | "selected">("all");
  const [viewMode, setViewMode] = useState<ViewMode>("flat");
  // Initialize with all file type categories collapsed by default
  const [collapsedCategories, setCollapsedCategories] = useState<Set<FileType>>(new Set(categoryOrder));
  const [collapsedContentCategories, setCollapsedContentCategories] = useState<Set<string>>(new Set());
  const [contentCategoriesInitialized, setContentCategoriesInitialized] = useState(false);
  const sortMenuRef = useRef<HTMLDivElement>(null);
  const { data: documents, isLoading } = useDocuments();
  const { data: conversations, isLoading: conversationsLoading } = useConversations();
  const deleteConversation = useDeleteConversation();
  const categorizeAll = useCategorizeAllDocuments();

  // Close sort menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (sortMenuRef.current && !sortMenuRef.current.contains(event.target as Node)) {
        setShowSortMenu(false);
      }
    };
    if (showSortMenu) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [showSortMenu]);

  // Initialize content categories as collapsed when documents first load
  useEffect(() => {
    if (documents && documents.length > 0 && !contentCategoriesInitialized) {
      const categories = new Set<string>();
      documents.forEach((doc) => {
        const category = (doc.metadata?.content_category as string) || "Uncategorized";
        categories.add(category);
      });
      setCollapsedContentCategories(categories);
      setContentCategoriesInitialized(true);
    }
  }, [documents, contentCategoriesInitialized]);

  const { data: stats } = useIngestionStats();
  const ingestFile = useIngestFile();
  const ingestDirectory = useIngestDirectory();
  const deleteDocument = useDeleteDocument();
  const reingestDocument = useReingestDocument();

  const handleAddFile = async () => {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: "Documents",
          extensions: ["pdf", "txt", "md", "mp4", "mkv", "avi", "mov", "webm", "mp3", "wav", "flac", "m4a", "png", "jpg", "jpeg"],
        },
      ],
    });

    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      for (const path of paths) {
        ingestFile.mutate(path);
      }
    }
  };

  const handleAddFolder = async () => {
    const selected = await open({
      directory: true,
    });

    if (selected && typeof selected === "string") {
      ingestDirectory.mutate({ path: selected, recursive: true });
    }
  };

  const getFileIcon = (fileType: FileType) => {
    switch (fileType) {
      case "pdf":
        return (
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
            <polyline points="14 2 14 8 20 8" />
            <text x="12" y="16" textAnchor="middle" fill="currentColor" stroke="none" fontSize="6" fontWeight="bold" fontFamily="system-ui">PDF</text>
          </svg>
        );
      case "text":
      case "markdown":
        return <FileText className="w-4 h-4" />;
      case "video":
        return <Film className="w-4 h-4" />;
      case "audio":
        return <Music className="w-4 h-4" />;
      case "image":
        return <Image className="w-4 h-4" />;
      case "screenshot":
        return <Camera className="w-4 h-4" />;
      default:
        return <FileText className="w-4 h-4" />;
    }
  };

  const getStatusIcon = (doc: Document) => {
    switch (doc.status) {
      case "processing":
        return <Loader2 className="w-4 h-4 animate-spin text-blue-400" />;
      case "completed":
        return <CheckCircle className="w-4 h-4 text-green-400" />;
      case "failed":
        return <XCircle className="w-4 h-4 text-red-400" />;
      default:
        return null;
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  };

  // Sort documents based on selected option
  // Note: Backend returns documents sorted by updated_at DESC, so "recent" uses original order
  const sortedDocuments = documents
    ? sortBy === "recent"
      ? documents // Already sorted by backend (most recent first)
      : [...documents].sort((a, b) => {
          switch (sortBy) {
            case "alphabetical":
              return a.title.toLowerCase().localeCompare(b.title.toLowerCase());
            case "type":
              // Sort by file type, then alphabetically within type
              const typeCompare = a.file_type.localeCompare(b.file_type);
              return typeCompare !== 0 ? typeCompare : a.title.toLowerCase().localeCompare(b.title.toLowerCase());
            case "size":
              // Sort by size descending (largest first)
              return b.file_size - a.file_size;
            default:
              return 0;
          }
        })
    : [];

  const handleDocumentCheckboxChange = (docId: string) => {
    if (selectedDocumentIds.includes(docId)) {
      onDocumentSelectionChange(selectedDocumentIds.filter((id) => id !== docId));
    } else {
      onDocumentSelectionChange([...selectedDocumentIds, docId]);
    }
  };

  const handleSelectAllDocuments = () => {
    if (documents) {
      if (selectedDocumentIds.length === documents.length) {
        onDocumentSelectionChange([]);
      } else {
        onDocumentSelectionChange(documents.map((d) => d.id));
      }
    }
  };

  const formatConversationTitle = (conv: Conversation) => {
    if (conv.title) {
      return conv.title.length > 30 ? conv.title.slice(0, 30) + "..." : conv.title;
    }
    return "New chat";
  };

  const toggleCategory = (category: FileType) => {
    setCollapsedCategories((prev) => {
      const next = new Set(prev);
      if (next.has(category)) {
        next.delete(category);
      } else {
        next.add(category);
      }
      return next;
    });
  };

  const toggleContentCategory = (category: string) => {
    setCollapsedContentCategories((prev) => {
      const next = new Set(prev);
      if (next.has(category)) {
        next.delete(category);
      } else {
        next.add(category);
      }
      return next;
    });
  };

  // Group documents by file type for grouped view
  const groupedDocuments = sortedDocuments.reduce((acc, doc) => {
    const type = doc.file_type;
    if (!acc[type]) {
      acc[type] = [];
    }
    acc[type].push(doc);
    return acc;
  }, {} as Record<FileType, Document[]>);

  // Get categories that have documents, in order
  const activeCategories = categoryOrder.filter((cat) => groupedDocuments[cat]?.length > 0);

  // Group documents by content category
  const contentGroupedDocuments = sortedDocuments.reduce((acc, doc) => {
    const category = (doc.metadata?.content_category as string) || "Uncategorized";
    if (!acc[category]) {
      acc[category] = [];
    }
    acc[category].push(doc);
    return acc;
  }, {} as Record<string, Document[]>);

  // Get content categories that have documents, sorted with Uncategorized last
  const activeContentCategories = Object.keys(contentGroupedDocuments).sort((a, b) => {
    if (a === "Uncategorized") return 1;
    if (b === "Uncategorized") return -1;
    return a.localeCompare(b);
  });

  // Check if any documents need categorization
  const uncategorizedCount = sortedDocuments.filter(
    (doc) => !doc.metadata?.content_category
  ).length;

  return (
    <div
      className={clsx(
        "flex flex-col bg-slate-800 border-r border-slate-700 transition-all duration-300",
        isExpanded ? "w-72" : "w-16"
      )}
    >
      {/* Header */}
      <div className={clsx(
        "flex items-center border-b border-slate-700",
        isExpanded ? "justify-between p-4" : "flex-col gap-2 p-2"
      )}>
        {isExpanded ? (
          <div className="flex items-center gap-2.5">
            <div className="relative flex-shrink-0">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-blue-600 flex items-center justify-center shadow-lg shadow-blue-500/20">
                <RecallIcon className="w-5 h-5 text-white" />
              </div>
              <div className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 bg-green-500 rounded-full border-2 border-slate-800" />
            </div>
            <div className="flex flex-col">
              <h1 className="text-base font-bold tracking-tight">
                <span className="text-white">RECALL</span>
                <span className="text-blue-400">.OS</span>
              </h1>
              <span className="text-[10px] text-slate-500 -mt-0.5">Personal AI Memory</span>
            </div>
          </div>
        ) : (
          <div className="relative flex-shrink-0">
            <div className="w-9 h-9 rounded-lg bg-gradient-to-br from-blue-500 to-blue-600 flex items-center justify-center shadow-lg shadow-blue-500/20">
              <RecallIcon className="w-5 h-5 text-white" />
            </div>
            <div className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 bg-green-500 rounded-full border-2 border-slate-800" />
          </div>
        )}
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className={clsx(
            "p-1.5 hover:bg-slate-700 rounded-lg transition-colors",
            !isExpanded && "w-9 h-9 flex items-center justify-center"
          )}
          title={isExpanded ? "Collapse sidebar" : "Expand sidebar"}
        >
          <ChevronRight
            className={clsx(
              "w-4 h-4 transition-transform",
              isExpanded && "rotate-180"
            )}
          />
        </button>
      </div>

      {/* Actions */}
      <div className={clsx(
        "p-2 border-b border-slate-700",
        !isExpanded && "flex flex-col items-center"
      )}>
        <button
          onClick={handleAddFile}
          className={clsx(
            "flex items-center gap-2 p-2 hover:bg-slate-700 rounded-lg transition-colors",
            isExpanded ? "w-full" : "w-9 h-9 justify-center"
          )}
          title="Add Files"
        >
          <FilePlus className="w-5 h-5 flex-shrink-0" />
          {isExpanded && <span>Add Files</span>}
        </button>
        <button
          onClick={handleAddFolder}
          className={clsx(
            "flex items-center gap-2 p-2 hover:bg-slate-700 rounded-lg transition-colors",
            isExpanded ? "w-full" : "w-9 h-9 justify-center"
          )}
          title="Add Folder"
        >
          <FolderPlus className="w-5 h-5 flex-shrink-0" />
          {isExpanded && <span>Add Folder</span>}
        </button>
      </div>

      {/* Stats */}
      {isExpanded && stats && (
        <div className="p-3 border-b border-slate-700 text-sm">
          <div className="flex items-center gap-2 text-slate-400">
            <Database className="w-4 h-4" />
            <span>
              {stats.total_documents} docs / {stats.total_chunks} chunks
            </span>
          </div>
          <div className="text-slate-500 text-xs mt-1">
            {formatSize(stats.total_size_bytes)} indexed
          </div>
        </div>
      )}

      {/* Ingestion Progress */}
      {ingestionProgress.length > 0 && isExpanded && (
        <div className="p-2 border-b border-slate-700">
          <div className="text-xs text-slate-400 mb-2">Processing...</div>
          {ingestionProgress.map((p) => (
            <div
              key={p.document_id}
              className="text-xs bg-slate-700/50 rounded p-2 mb-1"
            >
              <div className="truncate text-slate-300">
                {p.file_path.split(/[/\\]/).pop()}
              </div>
              <div className="flex items-center gap-2 mt-1">
                <div className="flex-1 bg-slate-600 rounded-full h-1.5">
                  <div
                    className="bg-blue-400 h-1.5 rounded-full transition-all"
                    style={{ width: `${p.progress * 100}%` }}
                  />
                </div>
                <span className="text-slate-400">{p.stage}</span>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Scrollable content area */}
      <div className="flex-1 overflow-y-auto">
        {/* Conversations Section */}
        {isExpanded && (
          <div className="p-2 border-b border-slate-700">
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs text-slate-400 uppercase tracking-wider">Chats</span>
              <button
                onClick={onNewConversation}
                className="p-1 hover:bg-slate-700 rounded transition-colors"
                title="New Chat"
              >
                <MessageSquarePlus className="w-4 h-4 text-slate-400" />
              </button>
            </div>
            {conversationsLoading ? (
              <div className="flex items-center justify-center py-2">
                <Loader2 className="w-4 h-4 animate-spin text-slate-400" />
              </div>
            ) : conversations && conversations.length > 0 ? (
              <div className="space-y-1 max-h-40 overflow-y-auto">
                {conversations.map((conv) => (
                  <div
                    key={conv.id}
                    className={clsx(
                      "group flex items-center gap-2 p-2 rounded-lg cursor-pointer transition-all",
                      currentConversationId === conv.id
                        ? "bg-blue-600/20 ring-1 ring-blue-500/50"
                        : "hover:bg-slate-700"
                    )}
                    onClick={() => onConversationSelect(conv.id)}
                  >
                    <MessageSquare className="w-4 h-4 text-slate-400 flex-shrink-0" />
                    <span className="flex-1 truncate text-sm">
                      {formatConversationTitle(conv)}
                    </span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteConversation.mutate(conv.id);
                        if (currentConversationId === conv.id) {
                          onConversationSelect(null);
                        }
                      }}
                      className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-600 rounded transition-all"
                      title="Delete conversation"
                    >
                      <Trash2 className="w-3 h-3 text-slate-400" />
                    </button>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-2 text-slate-500 text-xs">
                No conversations yet
              </div>
            )}
          </div>
        )}

        {/* Documents Section */}
        <div className="p-2">
          {isExpanded && (
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs text-slate-400 uppercase tracking-wider">Documents</span>
              <div className="flex items-center gap-1">
                <button
                  onClick={() => setFilterMode(filterMode === "all" ? "selected" : "all")}
                  className={clsx(
                    "flex items-center gap-1 px-2 py-0.5 rounded text-xs transition-colors",
                    filterMode === "selected"
                      ? "bg-blue-600/30 text-blue-400"
                      : "text-slate-400 hover:text-slate-300"
                  )}
                  title={filterMode === "all" ? "Filter by selected documents" : "Search all documents"}
                >
                  <Filter className="w-3 h-3" />
                  {filterMode === "selected" ? "Filtered" : "All"}
                </button>
              </div>
            </div>
          )}

          {/* Filter mode toggle info */}
          {isExpanded && filterMode === "selected" && (
            <div className="mb-2 px-2 py-1.5 bg-blue-600/10 rounded-lg border border-blue-500/20">
              <div className="flex items-center justify-between text-xs">
                <span className="text-blue-400">
                  {selectedDocumentIds.length} selected
                </span>
                {documents && documents.length > 0 && (
                  <button
                    onClick={handleSelectAllDocuments}
                    className="text-blue-400 hover:text-blue-300 underline"
                  >
                    {selectedDocumentIds.length === documents.length ? "Deselect all" : "Select all"}
                  </button>
                )}
              </div>
              <div className="mt-1.5 pt-1.5 border-t border-blue-500/20 flex items-center justify-between">
                <span className="text-slate-400 text-xs">Applies to new chats</span>
                <button
                  onClick={onNewConversation}
                  className="text-xs text-blue-400 hover:text-blue-300 underline"
                >
                  Start new chat
                </button>
              </div>
            </div>
          )}

          {/* Sort and View controls */}
          {isExpanded && documents && documents.length > 1 && (
            <div className="mb-2 flex items-center justify-between">
              <div className="relative" ref={sortMenuRef}>
                <button
                  onClick={() => setShowSortMenu(!showSortMenu)}
                  className="flex items-center gap-1.5 text-xs text-slate-400 hover:text-slate-300 transition-colors"
                >
                  <ArrowUpDown className="w-3.5 h-3.5" />
                  <span>Sort: {sortOptions.find(o => o.value === sortBy)?.label}</span>
                </button>
                {showSortMenu && (
                  <div className="absolute top-full left-0 mt-1 bg-slate-700 rounded-lg shadow-lg border border-slate-600 py-1 z-10 min-w-[120px]">
                    {sortOptions.map((option) => (
                      <button
                        key={option.value}
                        onClick={() => {
                          setSortBy(option.value);
                          setShowSortMenu(false);
                        }}
                        className={clsx(
                          "flex items-center gap-2 w-full px-3 py-1.5 text-xs text-left hover:bg-slate-600 transition-colors",
                          sortBy === option.value ? "text-blue-400" : "text-slate-300"
                        )}
                      >
                        {option.icon}
                        <span>{option.label}</span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
              {/* View mode toggle */}
              <div className="flex items-center gap-1 bg-slate-700/50 rounded-lg p-0.5">
                <button
                  onClick={() => setViewMode("flat")}
                  className={clsx(
                    "p-1 rounded transition-colors",
                    viewMode === "flat"
                      ? "bg-slate-600 text-blue-400"
                      : "text-slate-400 hover:text-slate-300"
                  )}
                  title="List view"
                >
                  <List className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={() => setViewMode("grouped")}
                  className={clsx(
                    "p-1 rounded transition-colors",
                    viewMode === "grouped"
                      ? "bg-slate-600 text-blue-400"
                      : "text-slate-400 hover:text-slate-300"
                  )}
                  title="Group by file type"
                >
                  <LayoutGrid className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={() => setViewMode("content")}
                  className={clsx(
                    "p-1 rounded transition-colors",
                    viewMode === "content"
                      ? "bg-slate-600 text-blue-400"
                      : "text-slate-400 hover:text-slate-300"
                  )}
                  title="Group by content topic"
                >
                  <FolderOpen className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          )}

          {/* Documents list */}
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="w-6 h-6 animate-spin text-slate-400" />
            </div>
          ) : sortedDocuments.length > 0 ? (
            viewMode === "flat" || !isExpanded ? (
              // Flat list view (also used for all views when collapsed)
              <div className={clsx(
                "space-y-1",
                !isExpanded && "flex flex-col items-center"
              )}>
                {sortedDocuments.map((doc) => {
                  const isHighlighted = highlightedDocIds.includes(doc.id);
                  const isSelected = selectedDocumentIds.includes(doc.id);
                  return (
                    <div
                      key={doc.id}
                      className={clsx(
                        "group flex items-center transition-all cursor-pointer",
                        isExpanded ? "gap-2 p-2 rounded-lg" : "w-9 h-9 justify-center rounded-lg",
                        isHighlighted
                          ? "bg-cyan-500/20 ring-1 ring-cyan-400/50 animate-pulse"
                          : filterMode === "selected" && isSelected
                          ? "bg-blue-600/10"
                          : "hover:bg-slate-700"
                      )}
                      title={doc.title}
                      onClick={() => {
                        if (filterMode === "selected") {
                          handleDocumentCheckboxChange(doc.id);
                        } else {
                          onDocumentClick?.(doc);
                        }
                      }}
                    >
                      {filterMode === "selected" && isExpanded && (
                        <div
                          className={clsx(
                            "w-4 h-4 rounded border flex items-center justify-center flex-shrink-0",
                            isSelected
                              ? "bg-blue-600 border-blue-600"
                              : "border-slate-500"
                          )}
                        >
                          {isSelected && <Check className="w-3 h-3 text-white" />}
                        </div>
                      )}
                      <span className={clsx(
                        "flex-shrink-0",
                        isHighlighted ? "text-cyan-400" : "text-slate-400"
                      )}>
                        {getFileIcon(doc.file_type)}
                      </span>
                      {isExpanded && (
                        <>
                          <span className="flex-1 truncate text-sm">{doc.title}</span>
                          {getStatusIcon(doc)}
                          {filterMode === "all" && (
                            <>
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  reingestDocument.mutate(doc.id);
                                }}
                                className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-600 rounded transition-all"
                                title="Re-process document"
                              >
                                <RefreshCw
                                  className={clsx(
                                    "w-3 h-3 text-slate-400",
                                    reingestDocument.isPending && "animate-spin"
                                  )}
                                />
                              </button>
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  deleteDocument.mutate(doc.id);
                                }}
                                className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-600 rounded transition-all"
                                title="Delete document"
                              >
                                <Trash2 className="w-3 h-3 text-slate-400" />
                              </button>
                            </>
                          )}
                        </>
                      )}
                    </div>
                  );
                })}
              </div>
            ) : viewMode === "grouped" ? (
              // Grouped by file type view
              <div className="space-y-2">
                {activeCategories.map((category) => {
                  const docs = groupedDocuments[category];
                  const isCollapsed = collapsedCategories.has(category);
                  const categorySelectedCount = docs.filter((d) =>
                    selectedDocumentIds.includes(d.id)
                  ).length;

                  return (
                    <div key={category}>
                      {/* Category header */}
                      <button
                        onClick={() => toggleCategory(category)}
                        className="flex items-center gap-2 w-full p-1.5 text-xs text-slate-400 hover:text-slate-300 transition-colors"
                      >
                        {isCollapsed ? (
                          <ChevronRight className="w-3.5 h-3.5" />
                        ) : (
                          <ChevronDown className="w-3.5 h-3.5" />
                        )}
                        <span className="text-slate-400">{getFileIcon(category)}</span>
                        <span className="font-medium">{categoryLabels[category]}</span>
                        <span className="text-slate-500">({docs.length})</span>
                        {filterMode === "selected" && categorySelectedCount > 0 && (
                          <span className="ml-auto text-blue-400">
                            {categorySelectedCount} selected
                          </span>
                        )}
                      </button>

                      {/* Category documents */}
                      {!isCollapsed && (
                        <div className="ml-3 space-y-1 border-l border-slate-700 pl-2">
                          {docs.map((doc) => {
                            const isHighlighted = highlightedDocIds.includes(doc.id);
                            const isSelected = selectedDocumentIds.includes(doc.id);
                            return (
                              <div
                                key={doc.id}
                                className={clsx(
                                  "group flex items-center gap-2 p-1.5 rounded-lg transition-all cursor-pointer",
                                  isHighlighted
                                    ? "bg-cyan-500/20 ring-1 ring-cyan-400/50 animate-pulse"
                                    : filterMode === "selected" && isSelected
                                    ? "bg-blue-600/10"
                                    : "hover:bg-slate-700"
                                )}
                                title={doc.file_path}
                                onClick={() => {
                                  if (filterMode === "selected") {
                                    handleDocumentCheckboxChange(doc.id);
                                  } else {
                                    onDocumentClick?.(doc);
                                  }
                                }}
                              >
                                {filterMode === "selected" && isExpanded && (
                                  <div
                                    className={clsx(
                                      "w-4 h-4 rounded border flex items-center justify-center flex-shrink-0",
                                      isSelected
                                        ? "bg-blue-600 border-blue-600"
                                        : "border-slate-500"
                                    )}
                                  >
                                    {isSelected && <Check className="w-3 h-3 text-white" />}
                                  </div>
                                )}
                                {isExpanded && (
                                  <>
                                    <span className="flex-1 truncate text-sm">{doc.title}</span>
                                    {getStatusIcon(doc)}
                                    {filterMode === "all" && (
                                      <>
                                        <button
                                          onClick={(e) => {
                                            e.stopPropagation();
                                            reingestDocument.mutate(doc.id);
                                          }}
                                          className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-600 rounded transition-all"
                                          title="Re-process document"
                                        >
                                          <RefreshCw
                                            className={clsx(
                                              "w-3 h-3 text-slate-400",
                                              reingestDocument.isPending && "animate-spin"
                                            )}
                                          />
                                        </button>
                                        <button
                                          onClick={(e) => {
                                            e.stopPropagation();
                                            deleteDocument.mutate(doc.id);
                                          }}
                                          className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-600 rounded transition-all"
                                          title="Delete document"
                                        >
                                          <Trash2 className="w-3 h-3 text-slate-400" />
                                        </button>
                                      </>
                                    )}
                                  </>
                                )}
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            ) : (
              // Grouped by content topic view
              <div className="space-y-2">
                {/* Categorize button */}
                {uncategorizedCount > 0 && (
                  <button
                    onClick={() => categorizeAll.mutate()}
                    disabled={categorizeAll.isPending}
                    className={clsx(
                      "flex items-center gap-2 w-full p-2 rounded-lg text-xs transition-colors",
                      "bg-blue-600/20 hover:bg-blue-600/30 text-blue-400 border border-blue-500/30"
                    )}
                  >
                    {categorizeAll.isPending ? (
                      <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    ) : (
                      <Sparkles className="w-3.5 h-3.5" />
                    )}
                    <span>
                      {categorizeAll.isPending
                        ? "Categorizing..."
                        : `Categorize ${uncategorizedCount} document${uncategorizedCount > 1 ? "s" : ""}`}
                    </span>
                  </button>
                )}

                {activeContentCategories.map((category) => {
                  const docs = contentGroupedDocuments[category];
                  const isCollapsed = collapsedContentCategories.has(category);
                  const categorySelectedCount = docs.filter((d) =>
                    selectedDocumentIds.includes(d.id)
                  ).length;

                  return (
                    <div key={category}>
                      {/* Category header */}
                      <button
                        onClick={() => toggleContentCategory(category)}
                        className="flex items-center gap-2 w-full p-1.5 text-xs text-slate-400 hover:text-slate-300 transition-colors"
                      >
                        {isCollapsed ? (
                          <ChevronRight className="w-3.5 h-3.5" />
                        ) : (
                          <ChevronDown className="w-3.5 h-3.5" />
                        )}
                        <FolderOpen className="w-3.5 h-3.5 text-blue-400" />
                        <span className="font-medium">{category}</span>
                        <span className="text-slate-500">({docs.length})</span>
                        {filterMode === "selected" && categorySelectedCount > 0 && (
                          <span className="ml-auto text-blue-400">
                            {categorySelectedCount} selected
                          </span>
                        )}
                      </button>

                      {/* Category documents */}
                      {!isCollapsed && (
                        <div className="ml-3 space-y-1 border-l border-blue-500/30 pl-2">
                          {docs.map((doc) => {
                            const isHighlighted = highlightedDocIds.includes(doc.id);
                            const isSelected = selectedDocumentIds.includes(doc.id);
                            return (
                              <div
                                key={doc.id}
                                className={clsx(
                                  "group flex items-center gap-2 p-1.5 rounded-lg transition-all cursor-pointer",
                                  isHighlighted
                                    ? "bg-cyan-500/20 ring-1 ring-cyan-400/50 animate-pulse"
                                    : filterMode === "selected" && isSelected
                                    ? "bg-blue-600/10"
                                    : "hover:bg-slate-700"
                                )}
                                title={doc.file_path}
                                onClick={() => {
                                  if (filterMode === "selected") {
                                    handleDocumentCheckboxChange(doc.id);
                                  } else {
                                    onDocumentClick?.(doc);
                                  }
                                }}
                              >
                                {filterMode === "selected" && isExpanded && (
                                  <div
                                    className={clsx(
                                      "w-4 h-4 rounded border flex items-center justify-center flex-shrink-0",
                                      isSelected
                                        ? "bg-blue-600 border-blue-600"
                                        : "border-slate-500"
                                    )}
                                  >
                                    {isSelected && <Check className="w-3 h-3 text-white" />}
                                  </div>
                                )}
                                <span className={isHighlighted ? "text-cyan-400" : "text-slate-400"}>
                                  {getFileIcon(doc.file_type)}
                                </span>
                                {isExpanded && (
                                  <>
                                    <span className="flex-1 truncate text-sm">{doc.title}</span>
                                    {getStatusIcon(doc)}
                                    {filterMode === "all" && (
                                      <>
                                        <button
                                          onClick={(e) => {
                                            e.stopPropagation();
                                            reingestDocument.mutate(doc.id);
                                          }}
                                          className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-600 rounded transition-all"
                                          title="Re-process document"
                                        >
                                          <RefreshCw
                                            className={clsx(
                                              "w-3 h-3 text-slate-400",
                                              reingestDocument.isPending && "animate-spin"
                                            )}
                                          />
                                        </button>
                                        <button
                                          onClick={(e) => {
                                            e.stopPropagation();
                                            deleteDocument.mutate(doc.id);
                                          }}
                                          className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-600 rounded transition-all"
                                          title="Delete document"
                                        >
                                          <Trash2 className="w-3 h-3 text-slate-400" />
                                        </button>
                                      </>
                                    )}
                                  </>
                                )}
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )
          ) : (
            isExpanded && (
              <div className="text-center py-8 text-slate-500 text-sm">
                <p>No documents yet</p>
                <p className="mt-1">Add files or folders to get started</p>
              </div>
            )
          )}
        </div>
      </div>

      {/* License, Help, and Settings buttons */}
      <div className={clsx(
        "p-2 border-t border-slate-700",
        !isExpanded && "flex flex-col items-center"
      )}>
        <button
          onClick={onLicenseClick}
          className={clsx(
            "flex items-center gap-2 p-2 hover:bg-slate-700 rounded-lg transition-colors",
            isExpanded ? "w-full" : "w-9 h-9 justify-center"
          )}
          title={isLicensed ? "License (Active)" : `Trial: ${trialDocsUsed ?? 0}/${trialDocsLimit ?? 25} docs`}
        >
          <Shield className={clsx(
            "w-5 h-5 flex-shrink-0",
            isLicensed ? "text-green-400" : "text-amber-400"
          )} />
          {isExpanded && (
            <div className="flex-1 flex items-center justify-between">
              <span className="flex items-center gap-2">
                License
                <span className={clsx(
                  "text-xs px-1.5 py-0.5 rounded",
                  isLicensed
                    ? "bg-green-500/20 text-green-400"
                    : "bg-amber-500/20 text-amber-400"
                )}>
                  {isLicensed ? "Active" : "Trial"}
                </span>
              </span>
              {!isLicensed && trialDocsUsed !== undefined && trialDocsLimit !== undefined && (
                <span className={clsx(
                  "text-xs",
                  trialDocsUsed >= trialDocsLimit ? "text-red-400" : "text-slate-400"
                )}>
                  {trialDocsUsed}/{trialDocsLimit}
                </span>
              )}
            </div>
          )}
        </button>
        <button
          onClick={onHelpClick}
          className={clsx(
            "flex items-center gap-2 p-2 hover:bg-slate-700 rounded-lg transition-colors",
            isExpanded ? "w-full" : "w-9 h-9 justify-center"
          )}
          title="Help"
        >
          <HelpCircle className="w-5 h-5 flex-shrink-0" />
          {isExpanded && <span>Help</span>}
        </button>
        <button
          onClick={onSettingsClick}
          className={clsx(
            "flex items-center gap-2 p-2 hover:bg-slate-700 rounded-lg transition-colors",
            isExpanded ? "w-full" : "w-9 h-9 justify-center"
          )}
          title="Settings"
        >
          <Settings className="w-5 h-5 flex-shrink-0" />
          {isExpanded && <span>Settings</span>}
        </button>
      </div>
    </div>
  );
}
