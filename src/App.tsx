import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useQueryClient } from "@tanstack/react-query";
import Sidebar from "./components/Sidebar";
import ChatPanel from "./components/ChatPanel";
import SourcePanel from "./components/SourcePanel";
import SettingsModal from "./components/SettingsModal";
import HelpModal from "./components/HelpModal";
import LicenseModal from "./components/LicenseModal";
import { useSettings } from "./hooks/useSettings";
import { useLicenseStatus } from "./hooks/useLicense";
import type { Citation, SourceChunk, IngestionProgress, Document } from "./types";

interface Toast {
  id: string;
  type: "info" | "success" | "error";
  message: string;
}

function App() {
  const [selectedSource, setSelectedSource] = useState<SourceChunk | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [showLicense, setShowLicense] = useState(false);
  const [ingestionProgress, setIngestionProgress] = useState<IngestionProgress[]>([]);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [highlightedDocIds, setHighlightedDocIds] = useState<string[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null);
  const [selectedDocumentIds, setSelectedDocumentIds] = useState<string[]>([]);
  const { data: settings, isLoading: settingsLoading } = useSettings();
  const { data: licenseStatus } = useLicenseStatus();
  const queryClient = useQueryClient();

  const handleNewConversation = useCallback(() => {
    setCurrentConversationId(null);
  }, []);

  const handleConversationSelect = useCallback((id: string | null) => {
    setCurrentConversationId(id);
  }, []);

  const handleConversationIdChange = useCallback((id: string) => {
    setCurrentConversationId(id);
    // Refresh conversations list when a new conversation is created
    queryClient.invalidateQueries({ queryKey: ["conversations"] });
  }, [queryClient]);

  const handleDocumentSelectionChange = useCallback((ids: string[]) => {
    setSelectedDocumentIds(ids);
  }, []);

  const addToast = (type: Toast["type"], message: string) => {
    const id = Date.now().toString();
    setToasts((prev) => [...prev, { id, type, message }]);
    // Auto-dismiss after 4 seconds
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 4000);
    return id;
  };

  const removeToast = (id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  };

  // Hide to system tray on window close instead of quitting
  useEffect(() => {
    const appWindow = getCurrentWindow();
    const unlisten = appWindow.onCloseRequested(async (e) => {
      e.preventDefault();
      await appWindow.hide();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for auto-ingest events
  useEffect(() => {
    const unlistenComplete = listen<Document>("auto-ingest-complete", () => {
      // Refresh documents list immediately when a file is auto-ingested
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    });

    const unlistenDeleted = listen<string>("document-deleted", () => {
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    });

    // Listen for capture events with toast notifications
    let processingToastId: string | null = null;

    const unlistenCaptureComplete = listen<{
      document_id: string;
      generated_title?: string;
    }>("capture-complete", (event) => {
      // Refresh documents list when a capture is fully processed (OCR complete)
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
      // Remove processing toast and show success
      if (processingToastId) {
        removeToast(processingToastId);
        processingToastId = null;
      }
      // Use generated title if available, otherwise generic message
      const message = event.payload.generated_title
        ? `Indexed: ${event.payload.generated_title}`
        : "Screenshot captured and indexed";
      addToast("success", message);
    });

    const unlistenCaptureStarted = listen<{ document_id: string }>("capture-started", () => {
      // Refresh to show the new capture immediately (before OCR)
      queryClient.invalidateQueries({ queryKey: ["documents"] });
      // Show processing toast
      processingToastId = addToast("info", "Processing screenshot...");
    });

    const unlistenCaptureError = listen<{ error: string }>("capture-error", (event) => {
      // Remove processing toast and show error
      if (processingToastId) {
        removeToast(processingToastId);
        processingToastId = null;
      }
      addToast("error", `Capture failed: ${event.payload.error}`);
    });

    return () => {
      unlistenComplete.then((fn) => fn());
      unlistenDeleted.then((fn) => fn());
      unlistenCaptureComplete.then((fn) => fn());
      unlistenCaptureStarted.then((fn) => fn());
      unlistenCaptureError.then((fn) => fn());
    };
  }, [queryClient]);

  // Listen for ingestion progress events
  useEffect(() => {
    const unlisten = listen<IngestionProgress>("ingestion-progress", (event) => {
      setIngestionProgress((prev) => {
        const existing = prev.findIndex((p) => p.document_id === event.payload.document_id);
        if (existing >= 0) {
          const updated = [...prev];
          updated[existing] = event.payload;
          // Remove completed/failed items after delay
          if (event.payload.stage === "completed" || event.payload.stage === "failed") {
            setTimeout(() => {
              setIngestionProgress((current) =>
                current.filter((p) => p.document_id !== event.payload.document_id)
              );
            }, 3000);
          }
          return updated;
        }
        return [...prev, event.payload];
      });
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for ingestion progress cleared event (database reset)
  useEffect(() => {
    const unlisten = listen("ingestion-progress-cleared", () => {
      setIngestionProgress([]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Listen for highlight documents event (from notification clicks)
  useEffect(() => {
    const unlisten = listen<string[]>("highlight-documents", (event) => {
      setHighlightedDocIds(event.payload);
      // Clear highlights after 5 seconds
      setTimeout(() => {
        setHighlightedDocIds([]);
      }, 5000);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Show settings modal if no API key configured
  useEffect(() => {
    if (!settingsLoading && settings && !settings.gemini_api_key) {
      setShowSettings(true);
    }
  }, [settings, settingsLoading]);

  const handleCitationClick = (citation: Citation) => {
    setSelectedSource({
      chunk_id: citation.chunk_id,
      document_id: citation.document_id,
      document_title: citation.document_title,
      content: citation.content_snippet,
      page_number: citation.page_number,
      timestamp: citation.timestamp,
      relevance_score: citation.relevance_score,
      search_type: "hybrid",
    });
  };

  const handleDocumentClick = (document: Document) => {
    setSelectedSource({
      chunk_id: 0,
      document_id: document.id,
      document_title: document.title,
      content: "",
      page_number: 1,
      timestamp: null,
      relevance_score: 1.0,
      search_type: "hybrid",
    });
  };

  return (
    <div className="flex h-screen bg-slate-900">
      {/* Sidebar */}
      <Sidebar
        onSettingsClick={() => setShowSettings(true)}
        onHelpClick={() => setShowHelp(true)}
        onLicenseClick={() => setShowLicense(true)}
        ingestionProgress={ingestionProgress}
        onDocumentClick={handleDocumentClick}
        highlightedDocIds={highlightedDocIds}
        currentConversationId={currentConversationId}
        onConversationSelect={handleConversationSelect}
        onNewConversation={handleNewConversation}
        selectedDocumentIds={selectedDocumentIds}
        onDocumentSelectionChange={handleDocumentSelectionChange}
        isLicensed={licenseStatus?.is_valid ?? false}
        trialDocsUsed={licenseStatus?.documents_used ?? undefined}
        trialDocsLimit={licenseStatus?.documents_limit ?? undefined}
      />

      {/* Main content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Chat panel */}
        <div className={`flex-1 ${selectedSource ? "w-1/2" : "w-full"} transition-all duration-300`}>
          <ChatPanel
            onCitationClick={handleCitationClick}
            onSourceSelect={setSelectedSource}
            conversationId={currentConversationId}
            onConversationIdChange={handleConversationIdChange}
            selectedDocumentIds={selectedDocumentIds}
          />
        </div>

        {/* Source panel */}
        {selectedSource && (
          <div className="w-1/2 border-l border-slate-700 animate-slide-in">
            <SourcePanel
              source={selectedSource}
              onClose={() => setSelectedSource(null)}
            />
          </div>
        )}
      </div>

      {/* Settings modal */}
      {showSettings && (
        <SettingsModal onClose={() => setShowSettings(false)} />
      )}

      {/* Help modal */}
      {showHelp && (
        <HelpModal onClose={() => setShowHelp(false)} />
      )}

      {/* License modal */}
      {showLicense && (
        <LicenseModal onClose={() => setShowLicense(false)} />
      )}

      {/* Toast notifications */}
      {toasts.length > 0 && (
        <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
          {toasts.map((toast) => (
            <div
              key={toast.id}
              className={`flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg animate-slide-in ${
                toast.type === "success"
                  ? "bg-green-600"
                  : toast.type === "error"
                  ? "bg-red-600"
                  : "bg-blue-600"
              }`}
            >
              <span className="text-white text-sm">{toast.message}</span>
              <button
                onClick={() => removeToast(toast.id)}
                className="text-white/70 hover:text-white"
              >
                &times;
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default App;
