import { useState, useEffect, useRef, useCallback } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { ChevronLeft, ChevronRight, ZoomIn, ZoomOut, Loader2, AlertCircle } from "lucide-react";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";

pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url
).toString();

interface PdfViewerProps {
  filePath: string;
  pageNumber?: number | null;
  highlightText?: string;
}

// Normalize text - handle ligatures and common PDF extraction issues
function normalize(text: string): string {
  return text
    .toLowerCase()
    .normalize("NFKD")
    // Expand common ligatures that pdfjs renders correctly
    .replace(/ﬁ/g, "fi")
    .replace(/ﬂ/g, "fl")
    .replace(/ﬀ/g, "ff")
    .replace(/ﬃ/g, "ffi")
    .replace(/ﬄ/g, "ffl")
    .replace(/[\u0300-\u036f]/g, "")  // Remove diacritics
    .replace(/[^\w\s]/g, " ")          // Punctuation to space
    .replace(/\s+/g, " ")              // Collapse whitespace
    .trim();
}

// Clean chunk content - remove page markers
function cleanChunk(text: string): string {
  return text.replace(/---\s*Page\s*\d+\s*---/gi, " ").trim();
}

// Get words, filtering out very short ones that might be ligature artifacts
function getWords(text: string): string[] {
  return normalize(text)
    .split(/\s+/)
    .filter(w => w.length > 0);
}

// Get n-grams (character sequences) from text for fuzzy matching
function getNgrams(text: string, n: number): Set<string> {
  const normalized = normalize(text).replace(/\s+/g, ""); // Remove spaces for ngrams
  const ngrams = new Set<string>();
  for (let i = 0; i <= normalized.length - n; i++) {
    ngrams.add(normalized.slice(i, i + n));
  }
  return ngrams;
}

// Calculate Jaccard similarity between two sets
function jaccardSimilarity(a: Set<string>, b: Set<string>): number {
  const intersection = new Set([...a].filter(x => b.has(x)));
  const union = new Set([...a, ...b]);
  return union.size > 0 ? intersection.size / union.size : 0;
}

interface SpanData {
  element: HTMLSpanElement;
  text: string;
  normalizedText: string;
  startIdx: number;
  endIdx: number;
}

export default function PdfViewer({ filePath, pageNumber, highlightText }: PdfViewerProps) {
  const [numPages, setNumPages] = useState<number>(0);
  const [currentPage, setCurrentPage] = useState<number>(1);
  const [scale, setScale] = useState<number>(1.0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [highlightPending, setHighlightPending] = useState(false);
  const [matchInfo, setMatchInfo] = useState<string>("");
  const containerRef = useRef<HTMLDivElement>(null);
  const highlightTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const observerRef = useRef<MutationObserver | null>(null);

  const fileUrl = convertFileSrc(filePath);

  useEffect(() => {
    if (pageNumber && pageNumber > 0) {
      if (numPages === 0) {
        setCurrentPage(pageNumber);
      } else if (pageNumber <= numPages) {
        setCurrentPage(pageNumber);
      }
    }
  }, [pageNumber, numPages]);

  const applyHighlights = useCallback((scrollToFirst = true) => {
    if (!highlightText || !containerRef.current) {
      setHighlightPending(false);
      return;
    }

    const textLayers = containerRef.current.querySelectorAll(".react-pdf__Page__textContent");
    if (!textLayers || textLayers.length === 0) {
      return;
    }

    const cleanedChunk = cleanChunk(highlightText);
    const chunkNgrams = getNgrams(cleanedChunk, 4); // Use 4-grams for matching
    const chunkWords = getWords(cleanedChunk);
    const chunkWordSet = new Set(chunkWords.filter(w => w.length > 4));

    if (chunkWords.length < 3) {
      setHighlightPending(false);
      setMatchInfo("Chunk too short");
      return;
    }

    let firstHighlight: HTMLSpanElement | null = null;
    let highlightedCount = 0;

    textLayers.forEach((layer) => {
      const spans = layer.querySelectorAll("span");

      // Collect span data
      const spanDataList: SpanData[] = [];
      let idx = 0;

      spans.forEach((span) => {
        const el = span as HTMLSpanElement;
        el.style.backgroundColor = "";
        el.style.borderRadius = "";
        el.classList.remove("highlight");

        const text = el.textContent || "";
        if (text.trim().length > 0) {
          spanDataList.push({
            element: el,
            text: text,
            normalizedText: normalize(text),
            startIdx: idx,
            endIdx: idx + 1
          });
          idx++;
        }
      });

      if (spanDataList.length === 0) return;

      // Build page text
      const pageText = spanDataList.map(s => s.text).join(" ");
      const pageNgrams = getNgrams(pageText, 4);
      const pageWords = getWords(pageText);

      // Calculate overall similarity using ngrams
      const ngramSimilarity = jaccardSimilarity(chunkNgrams, pageNgrams);

      // Find common words
      const commonWords = new Set<string>();
      for (const word of chunkWordSet) {
        if (pageWords.includes(word)) {
          commonWords.add(word);
        }
      }

      setMatchInfo(`Similarity: ${(ngramSimilarity * 100).toFixed(1)}%, Common words: ${commonWords.size}`);

      // If very low similarity, this might not be the right page
      if (ngramSimilarity < 0.05 && commonWords.size < 3) {
        return;
      }

      // Strategy: Find windows of spans with highest concentration of matching content
      const windowSize = 15;
      let bestWindowStart = -1;
      let bestWindowScore = 0;

      for (let i = 0; i <= spanDataList.length - windowSize; i++) {
        const windowSpans = spanDataList.slice(i, i + windowSize);
        const windowText = windowSpans.map(s => s.text).join(" ");
        const windowNgrams = getNgrams(windowText, 4);

        // Score based on ngram overlap with chunk
        const windowSimilarity = jaccardSimilarity(windowNgrams, chunkNgrams);

        // Boost score if window contains common words
        const windowWords = getWords(windowText);
        let wordBoost = 0;
        for (const word of windowWords) {
          if (commonWords.has(word)) wordBoost += 0.02;
        }

        const totalScore = windowSimilarity + wordBoost;

        if (totalScore > bestWindowScore) {
          bestWindowScore = totalScore;
          bestWindowStart = i;
        }
      }

      // If we found a good window, expand it to find the best contiguous region
      if (bestWindowStart >= 0 && bestWindowScore > 0.08) {
        // Find the actual bounds by looking for spans with matching content
        let startIdx = bestWindowStart;
        let endIdx = Math.min(bestWindowStart + windowSize, spanDataList.length);

        // Expand start backwards if spans have matching words
        while (startIdx > 0) {
          const prevSpan = spanDataList[startIdx - 1];
          const prevWords = getWords(prevSpan.text);
          const hasMatch = prevWords.some(w => commonWords.has(w) || chunkWordSet.has(w));
          if (hasMatch) {
            startIdx--;
          } else {
            break;
          }
        }

        // Expand end forwards
        while (endIdx < spanDataList.length) {
          const nextSpan = spanDataList[endIdx];
          const nextWords = getWords(nextSpan.text);
          const hasMatch = nextWords.some(w => commonWords.has(w) || chunkWordSet.has(w));
          if (hasMatch) {
            endIdx++;
          } else {
            break;
          }
        }

        // Highlight the region
        for (let i = startIdx; i < endIdx; i++) {
          const span = spanDataList[i];
          span.element.style.backgroundColor = "rgba(6, 182, 212, 0.35)";
          span.element.style.borderRadius = "2px";
          span.element.classList.add("highlight");
          highlightedCount++;

          if (!firstHighlight) {
            firstHighlight = span.element;
          }
        }
      } else if (commonWords.size >= 5) {
        // Fallback: highlight spans containing common words
        for (const span of spanDataList) {
          const spanWords = getWords(span.text);
          const hasCommon = spanWords.some(w => commonWords.has(w));
          if (hasCommon) {
            span.element.style.backgroundColor = "rgba(6, 182, 212, 0.25)";
            span.element.style.borderRadius = "2px";
            span.element.classList.add("highlight");
            highlightedCount++;

            if (!firstHighlight) {
              firstHighlight = span.element;
            }
          }
        }
      }
    });

    setHighlightPending(false);

    if (scrollToFirst && firstHighlight) {
      requestAnimationFrame(() => {
        firstHighlight?.scrollIntoView({
          behavior: 'smooth',
          block: 'center',
          inline: 'nearest'
        });
      });
    }
  }, [highlightText]);

  const setupHighlightObserver = useCallback(() => {
    if (!containerRef.current || !highlightText) return;

    if (observerRef.current) {
      observerRef.current.disconnect();
    }

    setHighlightPending(true);

    const existingTextLayer = containerRef.current.querySelector(".react-pdf__Page__textContent");
    if (existingTextLayer && existingTextLayer.children.length > 0) {
      applyHighlights(true);
      return;
    }

    observerRef.current = new MutationObserver(() => {
      const textLayer = containerRef.current?.querySelector(".react-pdf__Page__textContent");
      if (textLayer && textLayer.children.length > 0) {
        observerRef.current?.disconnect();
        applyHighlights(true);
      }
    });

    observerRef.current.observe(containerRef.current, {
      childList: true,
      subtree: true
    });

    highlightTimeoutRef.current = setTimeout(() => {
      observerRef.current?.disconnect();
      applyHighlights(true);
    }, 500);
  }, [highlightText, applyHighlights]);

  useEffect(() => {
    setupHighlightObserver();
    return () => {
      if (highlightTimeoutRef.current) {
        clearTimeout(highlightTimeoutRef.current);
      }
      if (observerRef.current) {
        observerRef.current.disconnect();
      }
    };
  }, [currentPage, loading, setupHighlightObserver]);

  const onDocumentLoadSuccess = useCallback(
    ({ numPages: pages }: { numPages: number }) => {
      setNumPages(pages);
      setLoading(false);
      setError(null);

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
    setupHighlightObserver();
  }, [setupHighlightObserver]);

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
      <div className="flex items-center justify-between px-4 py-2 bg-slate-800 border-b border-slate-700 shrink-0">
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

      {highlightText && (
        <div className="px-4 py-2 bg-slate-800 border-t border-slate-700 shrink-0">
          <div className="flex items-start gap-2">
            <span className="text-xs text-cyan-400 shrink-0 mt-0.5 flex items-center gap-1">
              {highlightPending && <Loader2 className="w-3 h-3 animate-spin" />}
              Source:
            </span>
            <p className="text-xs text-slate-400 line-clamp-2">
              {cleanChunk(highlightText).slice(0, 120)}...
            </p>
          </div>
          {matchInfo && (
            <p className="text-xs text-cyan-500/70 mt-1">{matchInfo}</p>
          )}
        </div>
      )}
    </div>
  );
}
