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
        "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium",
        "bg-cyan-500/10 hover:bg-cyan-500/20 border border-cyan-500/20 hover:border-cyan-500/40",
        "text-slate-200 transition-all duration-200",
        "hover:shadow-lg hover:shadow-cyan-500/10"
      )}
    >
      {citation.timestamp !== null ? (
        <>
          <Film className="w-3.5 h-3.5 text-cyan-400" />
          <span className="max-w-[100px] truncate">{citation.document_title}</span>
          <span className="text-cyan-400 font-semibold">@{formatTimestamp(citation.timestamp)}</span>
        </>
      ) : citation.page_number !== null ? (
        <>
          <BookOpen className="w-3.5 h-3.5 text-cyan-400" />
          <span className="max-w-[120px] truncate">{citation.document_title}</span>
          <span className="text-slate-400 mx-1">—</span>
          <span className="text-cyan-400 font-semibold">Page {citation.page_number}</span>
        </>
      ) : (
        <>
          <FileText className="w-3.5 h-3.5 text-cyan-400" />
          <span className="max-w-[120px] truncate">{citation.document_title}</span>
        </>
      )}
    </button>
  );
}
