import React, { useState, useRef, useEffect, useCallback } from "react";
import { Send, Loader2, AlertCircle, Sparkles } from "lucide-react";
import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import { useQueryWithSources } from "../hooks/useRag";
import { useConversationMessages } from "../hooks/useConversations";
import CitationChip from "./CitationChip";
import type { Citation, SourceChunk } from "../types";
import clsx from "clsx";

interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  citations: Citation[];
  sources: SourceChunk[];
}

interface ChatPanelProps {
  onCitationClick: (citation: Citation) => void;
  onSourceSelect?: (source: SourceChunk | null) => void;
  conversationId: string | null;
  onConversationIdChange: (id: string) => void;
  selectedDocumentIds: string[];
}

export default function ChatPanel({
  onCitationClick,
  onSourceSelect: _onSourceSelect,
  conversationId,
  onConversationIdChange,
  selectedDocumentIds,
}: ChatPanelProps) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  // Ref to prevent double-submit from rapid clicks
  const isSubmittingRef = useRef(false);
  // Track the last loaded conversation ID to detect changes
  const lastLoadedConversationIdRef = useRef<string | null>(null);

  const queryMutation = useQueryWithSources();
  const { data: conversationMessages } = useConversationMessages(conversationId);

  // Load messages when conversation changes
  useEffect(() => {
    if (conversationId !== lastLoadedConversationIdRef.current) {
      lastLoadedConversationIdRef.current = conversationId;

      if (conversationId && conversationMessages) {
        // Convert DB messages to our Message format
        const loadedMessages: Message[] = conversationMessages.map((m) => ({
          id: m.id,
          role: m.role as "user" | "assistant",
          content: m.content,
          citations: m.citations,
          sources: [], // Historical messages don't have full sources
        }));
        setMessages(loadedMessages);
      } else if (!conversationId) {
        // New conversation - clear messages
        setMessages([]);
      }
    }
  }, [conversationId, conversationMessages]);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleSubmit = useCallback(async (e: React.FormEvent) => {
    e.preventDefault();
    // Prevent double-submit using ref (synchronous check)
    if (!input.trim() || queryMutation.isPending || isSubmittingRef.current) return;

    isSubmittingRef.current = true;

    const userMessage: Message = {
      id: crypto.randomUUID(),
      role: "user",
      content: input.trim(),
      citations: [],
      sources: [],
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput("");

    try {
      const response = await queryMutation.mutateAsync({
        query: userMessage.content,
        conversationId: conversationId || undefined,
        maxChunks: 20,
        documentIds: selectedDocumentIds.length > 0 ? selectedDocumentIds : undefined,
      });

      // Update conversation ID if this is a new conversation
      if (response.conversation_id !== conversationId) {
        onConversationIdChange(response.conversation_id);
        lastLoadedConversationIdRef.current = response.conversation_id;
      }

      const assistantMessage: Message = {
        id: crypto.randomUUID(),
        role: "assistant",
        content: response.answer,
        citations: response.citations,
        sources: response.sources,
      };

      setMessages((prev) => [...prev, assistantMessage]);
    } catch (error) {
      const errorMessage: Message = {
        id: crypto.randomUUID(),
        role: "assistant",
        content: `Error: ${error instanceof Error ? error.message : "Failed to get response"}`,
        citations: [],
        sources: [],
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      isSubmittingRef.current = false;
    }
  }, [input, queryMutation, conversationId, selectedDocumentIds, onConversationIdChange]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
  };

  const renderMessage = (message: Message) => {
    if (message.role === "user") {
      return (
        <div className="flex justify-end mb-4">
          <div className="max-w-[80%] bg-blue-600 text-white rounded-2xl rounded-br-md px-4 py-3">
            <p className="whitespace-pre-wrap">{message.content}</p>
          </div>
        </div>
      );
    }

    return (
      <div className="mb-4">
        <div className="max-w-[80%] bg-slate-700 rounded-2xl rounded-bl-md px-4 py-3">
          <div className="prose prose-invert max-w-none">
            <ReactMarkdown
              rehypePlugins={[rehypeSanitize]}
              components={markdownComponents}
            >
              {message.content}
            </ReactMarkdown>
          </div>

          {/* Citations */}
          {message.citations.length > 0 && (
            <div className="mt-3 pt-3 border-t border-slate-600">
              <div className="text-xs text-slate-400 mb-2">Sources:</div>
              <div className="flex flex-wrap gap-2">
                {message.citations.map((citation, index) => (
                  <CitationChip
                    key={index}
                    citation={citation}
                    onClick={() => onCitationClick(citation)}
                  />
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    );
  };

  // Markdown components with Tailwind styling
  const markdownComponents = {
    h1: ({ children }: { children?: React.ReactNode }) => (
      <h1 className="text-xl font-bold mb-2">{children}</h1>
    ),
    h2: ({ children }: { children?: React.ReactNode }) => (
      <h2 className="text-lg font-bold mb-2">{children}</h2>
    ),
    h3: ({ children }: { children?: React.ReactNode }) => (
      <h3 className="text-base font-bold mb-2">{children}</h3>
    ),
    p: ({ children }: { children?: React.ReactNode }) => (
      <p className="mb-3">{children}</p>
    ),
    ul: ({ children }: { children?: React.ReactNode }) => (
      <ul className="list-disc pl-5 mb-3">{children}</ul>
    ),
    ol: ({ children }: { children?: React.ReactNode }) => (
      <ol className="list-decimal pl-5 mb-3">{children}</ol>
    ),
    li: ({ children }: { children?: React.ReactNode }) => (
      <li>{children}</li>
    ),
    code: ({ className, children }: { className?: string; children?: React.ReactNode }) => {
      const isInline = !className;
      if (isInline) {
        return <code className="bg-slate-800 px-1 rounded">{children}</code>;
      }
      return <code className={className}>{children}</code>;
    },
    pre: ({ children }: { children?: React.ReactNode }) => (
      <pre className="bg-slate-800 rounded p-3 mb-3 overflow-x-auto">{children}</pre>
    ),
    strong: ({ children }: { children?: React.ReactNode }) => (
      <strong className="font-bold">{children}</strong>
    ),
    em: ({ children }: { children?: React.ReactNode }) => (
      <em className="italic">{children}</em>
    ),
  };

  return (
    <div className="flex flex-col h-full">
      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4">
        {messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-slate-500">
            <Sparkles className="w-12 h-12 mb-4 text-blue-400" />
            <h2 className="text-xl font-semibold text-slate-300 mb-2">
              Welcome to RECALL.OS
            </h2>
            <p className="text-center max-w-md">
              Ask questions about your documents. I'll search through your
              knowledge base and provide answers with citations.
            </p>
          </div>
        ) : (
          <>
            {messages.map((message) => (
              <div key={message.id}>{renderMessage(message)}</div>
            ))}
            {queryMutation.isPending && (
              <div className="flex items-center gap-2 text-slate-400 mb-4">
                <Loader2 className="w-5 h-5 animate-spin" />
                <span>Searching and analyzing...</span>
              </div>
            )}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Input */}
      <div className="border-t border-slate-700 p-4">
        {queryMutation.isError && (
          <div className="flex items-center gap-2 text-red-400 text-sm mb-3">
            <AlertCircle className="w-4 h-4" />
            <span>Failed to get response. Please try again.</span>
          </div>
        )}

        <form onSubmit={handleSubmit} className="flex gap-3">
          <textarea
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Ask a question about your documents..."
            className={clsx(
              "flex-1 bg-slate-800 border border-slate-600 rounded-xl px-4 py-3",
              "resize-none focus:outline-none focus:border-blue-500",
              "placeholder:text-slate-500"
            )}
            rows={1}
            style={{
              minHeight: "48px",
              maxHeight: "200px",
              height: "auto",
            }}
            onInput={(e) => {
              const target = e.target as HTMLTextAreaElement;
              target.style.height = "auto";
              target.style.height = `${Math.min(target.scrollHeight, 200)}px`;
            }}
          />
          <button
            type="submit"
            disabled={!input.trim() || queryMutation.isPending}
            className={clsx(
              "px-4 py-3 bg-blue-600 hover:bg-blue-500 rounded-xl transition-colors",
              "disabled:opacity-50 disabled:cursor-not-allowed"
            )}
          >
            {queryMutation.isPending ? (
              <Loader2 className="w-5 h-5 animate-spin" />
            ) : (
              <Send className="w-5 h-5" />
            )}
          </button>
        </form>

        <div className="text-xs text-slate-500 mt-2 text-center">
          Press Enter to send, Shift+Enter for new line
        </div>
      </div>
    </div>
  );
}
