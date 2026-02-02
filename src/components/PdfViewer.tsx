import { useState, useEffect, useRef, useCallback } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { ChevronLeft, ChevronRight, ZoomIn, ZoomOut, Loader2, AlertCircle } from "lucide-react";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";

// Set up PDF.js worker - use local copy from node_modules for security (no CDN)
pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url
).toString();

interface PdfViewerProps {
  filePath: string;
  pageNumber?: number | null;
  highlightText?: string;
}

// Normalize text for comparison (remove extra whitespace, lowercase)
function normalizeText(text: string): string {
  return text.toLowerCase().replace(/\s+/g, " ").trim();
}

// Find words that appear in the highlight text
function getHighlightWords(text: string): string[] {
  const normalized = normalizeText(text);
  // Get significant words (longer than 3 chars) for matching
  const words = normalized.split(/\s+/).filter((w) => w.length > 3);
  // Return unique words
  return [...new Set(words)];
}

export default function PdfViewer({ filePath, pageNumber, highlightText }: PdfViewerProps) {
  const [numPages, setNumPages] = useState<number>(0);
  const [currentPage, setCurrentPage] = useState<number>(1);
  const [scale, setScale] = useState<number>(1.2);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const highlightTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Convert file path to Tauri asset URL
  const fileUrl = convertFileSrc(filePath);

  // Navigate to specified page when it changes
  useEffect(() => {
    if (pageNumber && pageNumber > 0) {
      if (numPages === 0) {
        // Document not loaded yet, set page for when it loads
        setCurrentPage(pageNumber);
      } else if (pageNumber <= numPages) {
        setCurrentPage(pageNumber);
      }
    }
  }, [pageNumber, numPages]);

  // Apply text highlighting
  const applyHighlights = useCallback(() => {
    if (!highlightText || !containerRef.current) return;

    // Clear any pending highlight operation
    if (highlightTimeoutRef.current) {
      clearTimeout(highlightTimeoutRef.current);
    }

    // Delay to ensure text layer is rendered
    highlightTimeoutRef.current = setTimeout(() => {
      const textLayers = containerRef.current?.querySelectorAll(".react-pdf__Page__textContent");
      if (!textLayers || textLayers.length === 0) return;

      const highlightWords = getHighlightWords(highlightText);
      if (highlightWords.length === 0) return;

      textLayers.forEach((layer) => {
        const spans = layer.querySelectorAll("span");
        spans.forEach((span) => {
          const spanElement = span as HTMLSpanElement;
          // Reset previous highlights
          spanElement.style.backgroundColor = "";
          spanElement.classList.remove("highlight");

          const spanText = normalizeText(spanElement.textContent || "");
          if (!spanText) return;

          // Check if any highlight word is in this span
          const hasMatch = highlightWords.some((word) => spanText.includes(word));

          if (hasMatch) {
            spanElement.style.backgroundColor = "rgba(59, 130, 246, 0.35)";
            spanElement.style.borderRadius = "2px";
            spanElement.classList.add("highlight");
          }
        });
      });
    }, 300);
  }, [highlightText]);

  // Apply highlights when page changes or loads
  useEffect(() => {
    applyHighlights();
    return () => {
      if (highlightTimeoutRef.current) {
        clearTimeout(highlightTimeoutRef.current);
      }
    };
  }, [currentPage, loading, applyHighlights]);

  const onDocumentLoadSuccess = useCallback(
    ({ numPages: pages }: { numPages: number }) => {
      setNumPages(pages);
      setLoading(false);
      setError(null);

      // Navigate to specified page after load
      if (pageNumber && pageNumber > 0 && pageNumber <= pages) {
        setCurrentPage(pageNumber);
      }
    },
    [pageNumber]
  );

  const onDocumentLoadError = useCallback((err: Error) => {
    console.error("PDF load error:", err);
    setError(err.message);
    setLoading(false);
  }, []);

  const onPageLoadSuccess = useCallback(() => {
    // Re-apply highlights after page renders
    applyHighlights();
  }, [applyHighlights]);

  // Handle clicks on PDF links - open in external browser
  const handlePdfClick = useCallback(async (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    const link = target.closest('a');
    if (link?.href?.startsWith('http')) {
      e.preventDefault();
      e.stopPropagation();
      await open(link.href);
    }
  }, []);

  const goToPrevPage = () => setCurrentPage((prev) => Math.max(1, prev - 1));
  const goToNextPage = () => setCurrentPage((prev) => Math.min(numPages, prev + 1));
  const zoomIn = () => setScale((prev) => Math.min(2.5, prev + 0.2));
  const zoomOut = () => setScale((prev) => Math.max(0.5, prev - 0.2));

  const goToPage = (e: React.ChangeEvent<HTMLInputElement>) => {
    const page = parseInt(e.target.value, 10);
    if (isFinite(page) && page >= 1 && page <= numPages) {
      setCurrentPage(page);
    }
  };

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-slate-400 p-8">
        <AlertCircle className="w-12 h-12 text-red-400 mb-4" />
        <p className="text-red-400 font-medium mb-2">Failed to load PDF</p>
        <p className="text-sm text-center mb-4 max-w-md">{error}</p>
        <div className="text-xs text-slate-500 bg-slate-800 p-3 rounded-lg max-w-full overflow-x-auto">
          <p className="font-medium mb-1">File path:</p>
          <code className="text-slate-400">{filePath}</code>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-slate-900">
      {/* Toolbar */}
      <div className="flex items-center justify-between px-4 py-2 bg-slate-800 border-b border-slate-700 shrink-0">
        {/* Page Navigation */}
        <div className="flex items-center gap-1">
          <button
            onClick={goToPrevPage}
            disabled={currentPage <= 1}
            className="p-1.5 hover:bg-slate-700 rounded disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            title="Previous page"
          >
            <ChevronLeft className="w-4 h-4" />
          </button>

          <div className="flex items-center gap-1 text-sm">
            <input
              type="number"
              min={1}
              max={numPages || 1}
              value={currentPage}
              onChange={goToPage}
              className="w-12 bg-slate-700 border border-slate-600 rounded px-2 py-1 text-center text-sm focus:outline-none focus:border-blue-500"
            />
            <span className="text-slate-400">/ {numPages || "..."}</span>
          </div>

          <button
            onClick={goToNextPage}
            disabled={currentPage >= numPages}
            className="p-1.5 hover:bg-slate-700 rounded disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            title="Next page"
          >
            <ChevronRight className="w-4 h-4" />
          </button>
        </div>

        {/* Zoom Controls */}
        <div className="flex items-center gap-1">
          <button
            onClick={zoomOut}
            disabled={scale <= 0.5}
            className="p-1.5 hover:bg-slate-700 rounded disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            title="Zoom out"
          >
            <ZoomOut className="w-4 h-4" />
          </button>
          <span className="text-sm text-slate-300 min-w-[50px] text-center">
            {Math.round(scale * 100)}%
          </span>
          <button
            onClick={zoomIn}
            disabled={scale >= 2.5}
            className="p-1.5 hover:bg-slate-700 rounded disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            title="Zoom in"
          >
            <ZoomIn className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* PDF Content */}
      <div ref={containerRef} className="flex-1 overflow-auto bg-slate-900/50" onClick={handlePdfClick}>
        <div className="min-h-full flex justify-center p-4">
          {loading && (
            <div className="flex flex-col items-center justify-center py-20">
              <Loader2 className="w-10 h-10 animate-spin text-blue-400 mb-4" />
              <p className="text-slate-400 text-sm">Loading PDF...</p>
            </div>
          )}

          <Document
            file={fileUrl}
            onLoadSuccess={onDocumentLoadSuccess}
            onLoadError={onDocumentLoadError}
            loading=""
            error=""
            className="flex flex-col items-center"
          >
            <Page
              key={`page_${currentPage}`}
              pageNumber={currentPage}
              scale={scale}
              renderTextLayer={true}
              renderAnnotationLayer={true}
              onLoadSuccess={onPageLoadSuccess}
              className="shadow-2xl"
              loading={
                <div
                  className="bg-white animate-pulse flex items-center justify-center"
                  style={{ width: 612 * scale, height: 792 * scale }}
                >
                  <Loader2 className="w-8 h-8 animate-spin text-slate-400" />
                </div>
              }
            />
          </Document>
        </div>
      </div>

      {/* Highlight Info Bar */}
      {highlightText && (
        <div className="px-4 py-2 bg-slate-800 border-t border-slate-700 shrink-0">
          <div className="flex items-start gap-2">
            <span className="text-xs text-blue-400 shrink-0 mt-0.5">Matching text:</span>
            <p className="text-xs text-slate-400 line-clamp-2">{highlightText}</p>
          </div>
        </div>
      )}
    </div>
  );
}
