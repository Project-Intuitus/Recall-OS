import { useState } from "react";
import { X, BookOpen, FileText, MessageSquare, Camera, Key, Settings2 } from "lucide-react";
import clsx from "clsx";

interface HelpModalProps {
  onClose: () => void;
}

type TabId = "getting-started" | "documents" | "chat" | "capture" | "api-key" | "advanced";

interface Tab {
  id: TabId;
  label: string;
  icon: React.ReactNode;
}

const tabs: Tab[] = [
  { id: "getting-started", label: "Getting Started", icon: <BookOpen className="w-4 h-4" /> },
  { id: "documents", label: "Documents", icon: <FileText className="w-4 h-4" /> },
  { id: "chat", label: "Chat & Search", icon: <MessageSquare className="w-4 h-4" /> },
  { id: "capture", label: "Screen Capture", icon: <Camera className="w-4 h-4" /> },
  { id: "api-key", label: "API Key Setup", icon: <Key className="w-4 h-4" /> },
  { id: "advanced", label: "Advanced Settings", icon: <Settings2 className="w-4 h-4" /> },
];

export default function HelpModal({ onClose }: HelpModalProps) {
  const [activeTab, setActiveTab] = useState<TabId>("getting-started");

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-slate-800 rounded-xl w-full max-w-3xl max-h-[80vh] flex flex-col shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-slate-700">
          <h2 className="text-lg font-semibold">Help & Reference</h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-slate-700 rounded transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="flex flex-1 overflow-hidden">
          {/* Tab navigation */}
          <div className="w-48 border-r border-slate-700 p-2 shrink-0">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={clsx(
                  "flex items-center gap-2 w-full px-3 py-2 rounded-lg text-left text-sm transition-colors",
                  activeTab === tab.id
                    ? "bg-blue-600 text-white"
                    : "hover:bg-slate-700 text-slate-300"
                )}
              >
                {tab.icon}
                <span>{tab.label}</span>
              </button>
            ))}
          </div>

          {/* Tab content */}
          <div className="flex-1 overflow-y-auto p-6">
            {activeTab === "getting-started" && <GettingStartedContent />}
            {activeTab === "documents" && <DocumentsContent />}
            {activeTab === "chat" && <ChatContent />}
            {activeTab === "capture" && <CaptureContent />}
            {activeTab === "api-key" && <ApiKeyContent />}
            {activeTab === "advanced" && <AdvancedSettingsContent />}
          </div>
        </div>
      </div>
    </div>
  );
}

function GettingStartedContent() {
  return (
    <div className="space-y-4">
      <h3 className="text-xl font-semibold text-blue-400">Welcome to RECALL.OS</h3>
      <p className="text-slate-300">
        RECALL.OS is a local-first AI-powered document retrieval system. It helps you search and chat with your documents using natural language, with full conversation history and smart document organization.
      </p>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Quick Start</h4>
        <ol className="list-decimal list-inside space-y-2 text-slate-300">
          <li>Set up your Gemini API key in Settings (required)</li>
          <li>Add documents by clicking "Add Files" or "Add Folder" in the sidebar</li>
          <li>Wait for documents to be processed (you'll see a progress indicator)</li>
          <li>Start chatting! Ask questions about your documents in the chat panel</li>
          <li>Your conversations are saved automatically - switch between them in the sidebar</li>
        </ol>
      </div>

      <div className="bg-slate-700/50 rounded-lg p-4 mt-4">
        <h4 className="font-medium text-white mb-2">Supported File Types</h4>
        <ul className="grid grid-cols-2 gap-2 text-sm text-slate-300">
          <li>PDF documents</li>
          <li>Text files (.txt)</li>
          <li>Markdown files (.md)</li>
          <li>Images (.png, .jpg)</li>
          <li>Videos (.mp4, .mkv, .avi)</li>
          <li>Audio (.mp3, .wav, .flac)</li>
        </ul>
      </div>

      <div className="bg-slate-700/50 rounded-lg p-4">
        <h4 className="font-medium text-white mb-2">Key Features</h4>
        <ul className="list-disc list-inside space-y-1 text-sm text-slate-300">
          <li>Conversation history - all chats are saved and searchable</li>
          <li>Document filtering - focus searches on specific documents</li>
          <li>Smart organization - view documents by type or content topic</li>
          <li>AI-powered categorization - auto-organize by content</li>
        </ul>
      </div>
    </div>
  );
}

function DocumentsContent() {
  return (
    <div className="space-y-4">
      <h3 className="text-xl font-semibold text-blue-400">Managing Documents</h3>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Adding Documents</h4>
        <ul className="list-disc list-inside space-y-2 text-slate-300">
          <li><strong>Add Files:</strong> Click to select individual files to import</li>
          <li><strong>Add Folder:</strong> Import all supported files from a folder</li>
          <li><strong>Watched Folders:</strong> Set up folders in Settings to auto-sync new files</li>
        </ul>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">View Modes</h4>
        <p className="text-slate-300 text-sm mb-2">Use the toggle buttons to switch between views:</p>
        <ul className="list-disc list-inside space-y-2 text-slate-300">
          <li><strong>List view:</strong> Simple flat list of all documents</li>
          <li><strong>Type view:</strong> Documents grouped by file type (PDF, Video, Audio, etc.)</li>
          <li><strong>Content view:</strong> Documents grouped by topic (Science, Business, etc.)</li>
        </ul>
        <p className="text-slate-400 text-sm mt-2">
          In Content view, click "Categorize documents" to have AI analyze and organize uncategorized files.
        </p>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Document Filtering</h4>
        <ul className="list-disc list-inside space-y-2 text-slate-300">
          <li>Click the <strong>"All"</strong> button to switch to <strong>"Filtered"</strong> mode</li>
          <li>Select specific documents using checkboxes</li>
          <li>Your chat will only search the selected documents</li>
          <li><strong>Note:</strong> Filtering applies to new conversations only</li>
        </ul>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Document Actions</h4>
        <ul className="list-disc list-inside space-y-2 text-slate-300">
          <li>Click a document to view it in the source panel</li>
          <li>Hover and click the refresh icon to re-process a document</li>
          <li>Hover and click the trash icon to delete a document</li>
        </ul>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Processing Status</h4>
        <ul className="list-disc list-inside space-y-2 text-slate-300">
          <li><span className="text-blue-400">Blue spinner:</span> Document is being processed</li>
          <li><span className="text-green-400">Green check:</span> Successfully indexed</li>
          <li><span className="text-red-400">Red X:</span> Processing failed</li>
        </ul>
      </div>
    </div>
  );
}

function ChatContent() {
  return (
    <div className="space-y-4">
      <h3 className="text-xl font-semibold text-blue-400">Chat & Search</h3>

      <div className="space-y-3">
        <h4 className="font-medium text-white">How It Works</h4>
        <p className="text-slate-300">
          RECALL.OS uses hybrid search combining vector similarity and keyword matching to find relevant content in your documents. When you ask a question, it retrieves the most relevant passages and uses AI to generate an answer with citations.
        </p>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Conversations</h4>
        <ul className="list-disc list-inside space-y-2 text-slate-300">
          <li>All conversations are automatically saved</li>
          <li>View your chat history in the <strong>Chats</strong> section of the sidebar</li>
          <li>Click a conversation to switch to it and continue where you left off</li>
          <li>Click the <strong>+</strong> button to start a new conversation</li>
          <li>Hover over a conversation and click the trash icon to delete it</li>
        </ul>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Document Filtering</h4>
        <p className="text-slate-300">
          Focus your search on specific documents by enabling filter mode in the Documents section. Select the documents you want to search, then start a new conversation. The AI will only look at your selected documents.
        </p>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Tips for Better Results</h4>
        <ul className="list-disc list-inside space-y-2 text-slate-300">
          <li>Be specific in your questions</li>
          <li>Reference document names if you're looking for specific content</li>
          <li>Use document filtering to narrow down search scope</li>
          <li>Click on citation chips to view the source passage</li>
          <li>The source panel shows the full context around the cited text</li>
        </ul>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Citations</h4>
        <p className="text-slate-300">
          Answers include clickable citation chips that show which documents were used. Click a citation to view the source in the right panel, with the relevant text highlighted.
        </p>
      </div>
    </div>
  );
}

function CaptureContent() {
  return (
    <div className="space-y-4">
      <h3 className="text-xl font-semibold text-blue-400">Screen Capture</h3>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Overview</h4>
        <p className="text-slate-300">
          Screen capture automatically takes screenshots at intervals and uses OCR to extract text, making your screen activity searchable.
        </p>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Settings</h4>
        <ul className="list-disc list-inside space-y-2 text-slate-300">
          <li><strong>Enable/Disable:</strong> Toggle automatic captures on or off</li>
          <li><strong>Capture Mode:</strong> Choose between active window or full screen</li>
          <li><strong>Interval:</strong> How often to take screenshots (in seconds)</li>
          <li><strong>Excluded Apps:</strong> Apps to ignore during capture</li>
        </ul>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Manual Capture</h4>
        <p className="text-slate-300">
          Use the global hotkey (default: Ctrl+Shift+S) to take a screenshot at any time. The hotkey can be customized in Settings.
        </p>
      </div>

      <div className="bg-slate-700/50 rounded-lg p-4 mt-4">
        <h4 className="font-medium text-white mb-2">Privacy Note</h4>
        <p className="text-sm text-slate-300">
          All screenshots are stored locally on your device. They are never uploaded to external servers. The Gemini API is only used for OCR text extraction.
        </p>
      </div>
    </div>
  );
}

function ApiKeyContent() {
  return (
    <div className="space-y-4">
      <h3 className="text-xl font-semibold text-blue-400">API Key Setup</h3>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Getting a Gemini API Key</h4>
        <ol className="list-decimal list-inside space-y-2 text-slate-300">
          <li>Go to <span className="text-blue-400">https://aistudio.google.com/apikey</span></li>
          <li>Sign in with your Google account</li>
          <li>Click "Create API Key"</li>
          <li>Copy the generated key</li>
          <li>Paste it in RECALL.OS Settings</li>
        </ol>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">API Usage</h4>
        <p className="text-slate-300">
          RECALL.OS uses the Gemini API for:
        </p>
        <ul className="list-disc list-inside space-y-2 text-slate-300">
          <li>Generating text embeddings for search</li>
          <li>OCR for images and scanned PDFs</li>
          <li>Answering questions about your documents</li>
        </ul>
      </div>

      <div className="bg-slate-700/50 rounded-lg p-4 mt-4">
        <h4 className="font-medium text-white mb-2">Free Tier</h4>
        <p className="text-sm text-slate-300">
          The Gemini API has a generous free tier that should be sufficient for personal use. Check Google's current pricing for the latest limits.
        </p>
      </div>

      <div className="bg-slate-700/50 rounded-lg p-4">
        <h4 className="font-medium text-white mb-2">Security</h4>
        <p className="text-sm text-slate-300">
          Your API key is stored locally and is never shared. All API calls go directly from your device to Google's servers.
        </p>
      </div>
    </div>
  );
}

function AdvancedSettingsContent() {
  return (
    <div className="space-y-4">
      <h3 className="text-xl font-semibold text-blue-400">Advanced Settings</h3>
      <p className="text-slate-300">
        These settings control how RECALL.OS processes and retrieves your documents. The defaults work well for most cases, but you can tune them for specific needs.
      </p>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Chunk Size (tokens)</h4>
        <p className="text-slate-300 text-sm">
          Controls how documents are split into searchable pieces. Each chunk is a segment of text that gets indexed separately.
        </p>
        <ul className="list-disc list-inside space-y-1 text-slate-400 text-sm ml-2">
          <li><strong>Default: 512 tokens</strong> (~400 words) - Good balance of context and precision</li>
          <li><strong>Smaller (256-384):</strong> More precise retrieval, but may lose context</li>
          <li><strong>Larger (768-1024):</strong> More context per chunk, but less precise matching</li>
        </ul>
        <p className="text-slate-500 text-xs mt-1">
          Tip: Use smaller chunks for technical documentation with many specific terms. Use larger chunks for narrative content.
        </p>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Chunk Overlap (tokens)</h4>
        <p className="text-slate-300 text-sm">
          How much text overlaps between adjacent chunks. Overlap ensures that information at chunk boundaries isn't lost.
        </p>
        <ul className="list-disc list-inside space-y-1 text-slate-400 text-sm ml-2">
          <li><strong>Default: 50 tokens</strong> - Prevents boundary issues without too much redundancy</li>
          <li><strong>Lower (20-30):</strong> Less storage, but may miss context at boundaries</li>
          <li><strong>Higher (100-150):</strong> Better context continuity, but more storage and processing</li>
        </ul>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Max Context Chunks</h4>
        <p className="text-slate-300 text-sm">
          Maximum number of document chunks included when generating an answer. More chunks = more context for the AI, but also higher API costs and potentially slower responses.
        </p>
        <ul className="list-disc list-inside space-y-1 text-slate-400 text-sm ml-2">
          <li><strong>Default: 20 chunks</strong> - Balanced context and performance</li>
          <li><strong>Lower (5-10):</strong> Faster responses, focused answers, lower API usage</li>
          <li><strong>Higher (30-50):</strong> More comprehensive answers, but may include less relevant content</li>
        </ul>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Video Segment Duration (seconds)</h4>
        <p className="text-slate-300 text-sm">
          When processing videos, RECALL.OS transcribes the audio in segments. This setting controls segment length.
        </p>
        <ul className="list-disc list-inside space-y-1 text-slate-400 text-sm ml-2">
          <li><strong>Default: 300 seconds</strong> (5 minutes) - Good for most videos</li>
          <li><strong>Shorter (60-120):</strong> More granular timestamps, but slower processing</li>
          <li><strong>Longer (600+):</strong> Faster processing, but less precise time references</li>
        </ul>
      </div>

      <div className="space-y-3">
        <h4 className="font-medium text-white">Folder Sync (Auto-Import)</h4>
        <p className="text-slate-300 text-sm">
          Found in the Sync tab. When enabled, RECALL.OS watches your selected folders and automatically imports new or modified files.
        </p>
        <ul className="list-disc list-inside space-y-1 text-slate-400 text-sm ml-2">
          <li>Add folders you want to keep synced (e.g., your research folder)</li>
          <li>New files are automatically indexed when detected</li>
          <li>Toggle off to pause syncing without removing folders</li>
        </ul>
      </div>

      <div className="bg-red-900/20 border border-red-500/30 rounded-lg p-4 mt-4">
        <h4 className="font-medium text-red-400 mb-2">Danger Zone: Reset Database</h4>
        <p className="text-sm text-slate-300">
          This permanently deletes all your indexed documents, conversation history, and settings. Use only if you want to start completely fresh. This action cannot be undone.
        </p>
      </div>

      <div className="bg-slate-700/50 rounded-lg p-4">
        <h4 className="font-medium text-white mb-2">When to Change Settings</h4>
        <p className="text-sm text-slate-300">
          Changes to chunk size and overlap only affect newly ingested documents. To apply new settings to existing documents, you'll need to re-process them using the refresh button on each document.
        </p>
      </div>
    </div>
  );
}
