import { useEffect, useState, useRef, useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize, PhysicalPosition } from "@tauri-apps/api/dpi";
import { invoke } from "@tauri-apps/api/core";

interface RelatedDocument {
  id: string;
  title: string;
  similarity: number;
}

interface NotificationData {
  title: string;
  message: string;
  documentId?: string;
  relatedDocuments?: RelatedDocument[];
}

const NOTIFICATION_WIDTH = 340;
const NOTIFICATION_PADDING = 16; // margin around content

export default function NotificationWindow() {
  const [notification, setNotification] = useState<NotificationData | null>(null);
  const [isVisible, setIsVisible] = useState(false);
  const [isClosing, setIsClosing] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);

  // Resize window to fit content
  const resizeToFit = useCallback(async () => {
    if (!contentRef.current) return;

    const height = contentRef.current.offsetHeight + NOTIFICATION_PADDING;
    const width = NOTIFICATION_WIDTH + NOTIFICATION_PADDING;

    try {
      const appWindow = getCurrentWindow();

      // Get current position before resize
      const currentPos = await appWindow.outerPosition();
      const currentSize = await appWindow.outerSize();

      // Resize the window
      await appWindow.setSize(new LogicalSize(width, height));

      // Adjust Y position to keep bottom-right alignment (move up if height increased)
      const heightDiff = height - currentSize.height;
      if (heightDiff !== 0) {
        const newY = currentPos.y - heightDiff;
        await appWindow.setPosition(new PhysicalPosition(currentPos.x, newY));
      }
    } catch (e) {
      console.error("Failed to resize window:", e);
    }
  }, []);

  useEffect(() => {
    invoke<NotificationData>("notification_window_ready")
      .then((data) => {
        setNotification(data);
        setIsVisible(true);
        setTimeout(() => handleClose(), 5000);
      })
      .catch(console.error);
  }, []);

  // Resize after notification data is loaded
  useEffect(() => {
    if (notification && contentRef.current) {
      // Small delay to ensure content is rendered
      requestAnimationFrame(() => {
        resizeToFit();
      });
    }
  }, [notification, resizeToFit]);

  const handleClose = async () => {
    setIsClosing(true);
    setTimeout(async () => {
      const appWindow = getCurrentWindow();
      await appWindow.close();
    }, 200);
  };

  const handleClick = async () => {
    try {
      // Collect IDs to highlight: the new document + all related documents
      const highlightIds: string[] = [];
      if (notification?.documentId) {
        highlightIds.push(notification.documentId);
      }
      if (notification?.relatedDocuments) {
        highlightIds.push(...notification.relatedDocuments.map(d => d.id));
      }

      // Focus main window and pass highlight IDs
      await invoke("focus_main_window_with_highlights", { documentIds: highlightIds });
    } catch {
      // Fallback to just focusing
      try {
        await invoke("focus_main_window");
      } catch {
        // Ignore
      }
    }
    handleClose();
  };

  const relatedCount = notification?.relatedDocuments?.length ?? 0;

  return (
    <>
      <style>{`
        html, body, #root {
          background: transparent !important;
        }
        @keyframes slide-in {
          from { opacity: 0; transform: translateX(100%); }
          to { opacity: 1; transform: translateX(0); }
        }
        @keyframes slide-out {
          from { opacity: 1; transform: translateX(0); }
          to { opacity: 0; transform: translateX(100%); }
        }
        @keyframes shrink {
          from { width: 100%; }
          to { width: 0%; }
        }
        .animate-slide-in { animation: slide-in 0.3s ease-out forwards; }
        .animate-slide-out { animation: slide-out 0.2s ease-in forwards; }
        .animate-shrink { animation: shrink linear forwards; }
      `}</style>

      <div
        ref={contentRef}
        className={`
          m-2 bg-slate-800 rounded-xl overflow-hidden cursor-pointer flex flex-col
          ${isVisible && !isClosing ? "animate-slide-in" : ""}
          ${isClosing ? "animate-slide-out" : ""}
        `}
        style={{
          width: NOTIFICATION_WIDTH,
        }}
        onClick={handleClick}
        data-tauri-drag-region
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-2 bg-slate-900/50 border-b border-slate-700/50">
          <div className="flex items-center gap-2">
            <div className="w-4 h-4 rounded bg-gradient-to-br from-cyan-400 to-blue-500 flex items-center justify-center">
              <svg className="w-2.5 h-2.5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
            </div>
            <span className="text-xs font-medium text-slate-400 tracking-wide">RECALL.OS</span>
          </div>
          <button
            onClick={(e) => { e.stopPropagation(); handleClose(); }}
            className="text-slate-500 hover:text-slate-300 transition-colors p-1 -mr-1"
          >
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content */}
        {notification ? (
          <div className="px-4 py-3">
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 w-8 h-8 rounded-full bg-cyan-500/20 flex items-center justify-center">
                {relatedCount > 0 ? (
                  <svg className="w-4 h-4 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                  </svg>
                ) : notification?.title === "Processing Screenshot" ? (
                  <svg className="w-4 h-4 text-cyan-400 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                ) : (
                  <svg className="w-4 h-4 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
                  </svg>
                )}
              </div>
              <div className="flex-1 min-w-0">
                {relatedCount > 0 ? (
                  <>
                    <h3 className="text-sm font-semibold text-white truncate">
                      {relatedCount} {relatedCount === 1 ? "Match" : "Matches"} Found
                    </h3>
                    <p className="text-xs text-slate-400 mt-0.5 truncate">{notification.title}</p>
                  </>
                ) : (
                  <>
                    <h3 className="text-sm font-semibold text-white truncate">
                      {notification.title}
                    </h3>
                    <p className="text-xs text-slate-400 mt-0.5 truncate">{notification.message}</p>
                  </>
                )}
              </div>
            </div>

            {notification.relatedDocuments && notification.relatedDocuments.length > 0 && (
              <div className="mt-3 pt-3 border-t border-slate-700/50">
                <p className="text-xs text-slate-500 mb-1.5">Similar to:</p>
                <div className="flex flex-wrap gap-1.5">
                  {notification.relatedDocuments.slice(0, 3).map((doc) => (
                    <span key={doc.id} className="inline-flex items-center px-2 py-0.5 rounded text-xs bg-slate-700/50 text-slate-300">
                      {doc.title.length > 20 ? doc.title.slice(0, 20) + "..." : doc.title}
                    </span>
                  ))}
                  {notification.relatedDocuments.length > 3 && (
                    <span className="inline-flex items-center px-2 py-0.5 rounded text-xs text-slate-500">
                      +{notification.relatedDocuments.length - 3} more
                    </span>
                  )}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="px-4 py-3 flex items-center gap-2">
            <div className="w-4 h-4 rounded bg-gradient-to-br from-cyan-400 to-blue-500 animate-pulse" />
            <span className="text-xs text-slate-400">Loading...</span>
          </div>
        )}

        {/* Progress bar */}
        <div className="h-1 bg-slate-700 rounded-b-xl overflow-hidden">
          <div className="h-full bg-gradient-to-r from-cyan-500 to-blue-500 animate-shrink" style={{ animationDuration: "5s" }} />
        </div>
      </div>
    </>
  );
}
