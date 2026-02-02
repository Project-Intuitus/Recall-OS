import { useEffect, useState } from "react";
import { X, FileText, Sparkles } from "lucide-react";

export interface ToastData {
  id: string;
  type: "info" | "success" | "warning" | "related";
  title: string;
  message: string;
  relatedDocs?: { id: string; title: string; similarity: number }[];
  duration?: number;
  onDocumentClick?: (docId: string) => void;
}

interface ToastProps {
  toast: ToastData;
  onDismiss: (id: string) => void;
}

export function Toast({ toast, onDismiss }: ToastProps) {
  const [isExiting, setIsExiting] = useState(false);

  useEffect(() => {
    const duration = toast.duration ?? 8000;
    const timer = setTimeout(() => {
      setIsExiting(true);
      setTimeout(() => onDismiss(toast.id), 300);
    }, duration);

    return () => clearTimeout(timer);
  }, [toast.id, toast.duration, onDismiss]);

  const handleDismiss = () => {
    setIsExiting(true);
    setTimeout(() => onDismiss(toast.id), 300);
  };

  const bgColor = {
    info: "bg-blue-600/90",
    success: "bg-green-600/90",
    warning: "bg-yellow-600/90",
    related: "bg-purple-600/90",
  }[toast.type];

  return (
    <div
      className={`
        ${bgColor} backdrop-blur-sm rounded-lg shadow-lg p-4 max-w-sm
        transform transition-all duration-300 ease-out
        ${isExiting ? "opacity-0 translate-x-full" : "opacity-100 translate-x-0"}
      `}
    >
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0 mt-0.5">
          {toast.type === "related" ? (
            <Sparkles className="w-5 h-5 text-purple-200" />
          ) : (
            <FileText className="w-5 h-5 text-white/80" />
          )}
        </div>

        <div className="flex-1 min-w-0">
          <h4 className="text-sm font-semibold text-white">{toast.title}</h4>
          <p className="text-xs text-white/80 mt-1">{toast.message}</p>

          {toast.type === "related" && toast.relatedDocs && toast.relatedDocs.length > 0 && (
            <div className="mt-2 space-y-1">
              {toast.relatedDocs.slice(0, 3).map((doc) => (
                <button
                  key={doc.id}
                  onClick={() => {
                    toast.onDocumentClick?.(doc.id);
                    handleDismiss();
                  }}
                  className="flex items-center gap-2 text-xs text-purple-200 hover:text-white
                             transition-colors w-full text-left"
                >
                  <FileText className="w-3 h-3 flex-shrink-0" />
                  <span className="truncate">{doc.title}</span>
                  <span className="text-purple-300/60 flex-shrink-0">
                    {Math.round(doc.similarity * 100)}%
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>

        <button
          onClick={handleDismiss}
          className="flex-shrink-0 text-white/60 hover:text-white transition-colors"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}

interface ToastContainerProps {
  toasts: ToastData[];
  onDismiss: (id: string) => void;
}

export function ToastContainer({ toasts, onDismiss }: ToastContainerProps) {
  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      {toasts.map((toast) => (
        <Toast key={toast.id} toast={toast} onDismiss={onDismiss} />
      ))}
    </div>
  );
}
