import { FileText, Film, BookOpen } from "lucide-react";
import type { Citation } from "../types";
import clsx from "clsx";

interface CitationChipProps {
  citation: Citation;
  onClick: () => void;
}

export default function CitationChip({ citation, onClick }: CitationChipProps) {
  const formatTimestamp = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  };

  return (
    <button
      onClick={onClick}
      className={clsx(
        "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs",
        "bg-slate-600 hover:bg-slate-500 transition-colors",
        "text-slate-200"
      )}
    >
      {citation.timestamp !== null ? (
        <>
          <Film className="w-3 h-3" />
          <span className="max-w-[100px] truncate">{citation.document_title}</span>
          <span className="text-blue-400">@{formatTimestamp(citation.timestamp)}</span>
        </>
      ) : citation.page_number !== null ? (
        <>
          <BookOpen className="w-3 h-3" />
          <span className="max-w-[100px] truncate">{citation.document_title}</span>
          <span className="text-blue-400">p.{citation.page_number}</span>
        </>
      ) : (
        <>
          <FileText className="w-3 h-3" />
          <span className="max-w-[120px] truncate">{citation.document_title}</span>
        </>
      )}
    </button>
  );
}
