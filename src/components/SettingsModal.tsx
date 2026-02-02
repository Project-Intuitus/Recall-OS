import { useState, useEffect } from "react";
import { X, Key, Loader2, CheckCircle, AlertCircle, Settings2, FolderOpen, Trash2, Plus, Eye, EyeOff, RefreshCw, Clock, Camera } from "lucide-react";
import { useSettings, useUpdateSettings, useValidateApiKey, useClearApiKey, useGetApiKeyUnmasked } from "../hooks/useSettings";
import { useResetDatabase } from "../hooks/useDocuments";
import { useWatcherStatus, useAddWatchedFolder, useRemoveWatchedFolder, useToggleAutoIngest } from "../hooks/useWatcher";
import { open } from "@tauri-apps/plugin-dialog";
import type { Settings } from "../types";
import clsx from "clsx";
import ScreenCaptureSettings from "./ScreenCaptureSettings";

interface SettingsModalProps {
  onClose: () => void;
}

export default function SettingsModal({ onClose }: SettingsModalProps) {
  const { data: settings, isLoading } = useSettings();
  const { data: watcherStatus } = useWatcherStatus();
  const updateSettings = useUpdateSettings();
  const validateApiKey = useValidateApiKey();
  const addWatchedFolder = useAddWatchedFolder();
  const removeWatchedFolder = useRemoveWatchedFolder();
  const toggleAutoIngest = useToggleAutoIngest();

  const [apiKey, setApiKey] = useState("");
  const [showApiKey, setShowApiKey] = useState(false);
  const [localSettings, setLocalSettings] = useState<Partial<Settings>>({});
  const [activeTab, setActiveTab] = useState<"api" | "watch" | "capture" | "advanced">("api");
  const [pendingFolders, setPendingFolders] = useState<string[]>([]);
  const [showResetConfirm, setShowResetConfirm] = useState(false);
  const [showRemoveKeyConfirm, setShowRemoveKeyConfirm] = useState(false);

  const clearApiKey = useClearApiKey();
  const getApiKeyUnmasked = useGetApiKeyUnmasked();
  const resetDatabase = useResetDatabase();

  useEffect(() => {
    if (settings) {
      setLocalSettings(settings);
      setApiKey(settings.gemini_api_key || "");
    }
  }, [settings]);

  const handleValidateKey = async () => {
    if (!apiKey.trim() || apiKey.startsWith("****")) return;

    try {
      const isValid = await validateApiKey.mutateAsync(apiKey);
      if (isValid) {
        setLocalSettings((prev) => ({ ...prev, gemini_api_key: apiKey }));
      }
    } catch (error) {
      console.error("API key validation failed:", error);
    }
  };

  const handleSaveSettings = async () => {
    if (!settings) return;

    const updatedSettings: Settings = {
      ...settings,
      ...localSettings,
      gemini_api_key: apiKey.startsWith("****") ? settings.gemini_api_key : apiKey,
    };

    await updateSettings.mutateAsync(updatedSettings);

    // Add pending folders
    for (const folder of pendingFolders) {
      await addWatchedFolder.mutateAsync(folder);
    }
    setPendingFolders([]);

    onClose();
  };

  const handleAddPendingFolder = async () => {
    const folder = await open({
      directory: true,
      multiple: false,
      title: "Select folder to sync",
    });
    if (folder && typeof folder === "string") {
      // Check if folder is already in watched or pending
      const alreadyWatched = watcherStatus?.watched_folders.includes(folder);
      const alreadyPending = pendingFolders.includes(folder);
      if (!alreadyWatched && !alreadyPending) {
        setPendingFolders((prev) => [...prev, folder]);
      }
    }
  };

  const handleRemovePendingFolder = (folder: string) => {
    setPendingFolders((prev) => prev.filter((f) => f !== folder));
  };

  const handleShowApiKey = async () => {
    if (showApiKey) {
      setShowApiKey(false);
    } else {
      try {
        const unmaskedKey = await getApiKeyUnmasked.mutateAsync();
        if (unmaskedKey) {
          setApiKey(unmaskedKey);
          setShowApiKey(true);
        }
      } catch (error) {
        console.error("Failed to get API key:", error);
      }
    }
  };

  const handleRemoveApiKey = async () => {
    try {
      await clearApiKey.mutateAsync();
      setApiKey("");
      setShowApiKey(false);
      setShowRemoveKeyConfirm(false);
    } catch (error) {
      console.error("Failed to remove API key:", error);
    }
  };

  const handleResetDatabase = async () => {
    console.log("handleResetDatabase called, attempting reset...");
    try {
      await resetDatabase.mutateAsync();
      console.log("Database reset successful");
      setShowResetConfirm(false);
    } catch (error) {
      console.error("Failed to reset database:", error);
      alert(`Failed to reset database: ${error}`);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 animate-fade-in p-4">
      <div className="bg-slate-800 rounded-xl w-full max-w-lg shadow-2xl animate-slide-in flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-slate-700 flex-shrink-0">
          <h2 className="text-lg font-semibold">Settings</h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-slate-700 rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-slate-700 flex-shrink-0 overflow-x-auto">
          <button
            onClick={() => setActiveTab("api")}
            className={clsx(
              "flex items-center gap-2 px-3 py-3 text-sm transition-colors whitespace-nowrap",
              activeTab === "api"
                ? "text-blue-400 border-b-2 border-blue-400"
                : "text-slate-400 hover:text-slate-300"
            )}
          >
            <Key className="w-4 h-4" />
            API
          </button>
          <button
            onClick={() => setActiveTab("watch")}
            className={clsx(
              "flex items-center gap-2 px-3 py-3 text-sm transition-colors whitespace-nowrap",
              activeTab === "watch"
                ? "text-blue-400 border-b-2 border-blue-400"
                : "text-slate-400 hover:text-slate-300"
            )}
          >
            <RefreshCw className="w-4 h-4" />
            Sync
          </button>
          <button
            onClick={() => setActiveTab("capture")}
            className={clsx(
              "flex items-center gap-2 px-3 py-3 text-sm transition-colors whitespace-nowrap",
              activeTab === "capture"
                ? "text-blue-400 border-b-2 border-blue-400"
                : "text-slate-400 hover:text-slate-300"
            )}
          >
            <Camera className="w-4 h-4" />
            <span className="hidden sm:inline">Screen</span> Capture
          </button>
          <button
            onClick={() => setActiveTab("advanced")}
            className={clsx(
              "flex items-center gap-2 px-3 py-3 text-sm transition-colors whitespace-nowrap",
              activeTab === "advanced"
                ? "text-blue-400 border-b-2 border-blue-400"
                : "text-slate-400 hover:text-slate-300"
            )}
          >
            <Settings2 className="w-4 h-4" />
            Advanced
          </button>
        </div>

        {/* Content */}
        <div className="p-4 overflow-y-auto flex-1 min-h-0">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="w-6 h-6 animate-spin" />
            </div>
          ) : activeTab === "api" ? (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">
                  Gemini API Key
                </label>
                <div className="flex gap-2">
                  <div className="relative flex-1">
                    <input
                      type={showApiKey ? "text" : "password"}
                      value={apiKey}
                      onChange={(e) => setApiKey(e.target.value)}
                      placeholder="Enter your Gemini API key"
                      className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 pr-10 focus:outline-none focus:border-blue-500"
                    />
                    {apiKey && apiKey.startsWith("****") && (
                      <button
                        onClick={handleShowApiKey}
                        disabled={getApiKeyUnmasked.isPending}
                        className="absolute right-2 top-1/2 -translate-y-1/2 p-1 hover:bg-slate-600 rounded transition-colors"
                        title={showApiKey ? "Hide API key" : "Show API key"}
                      >
                        {getApiKeyUnmasked.isPending ? (
                          <Loader2 className="w-4 h-4 animate-spin text-slate-400" />
                        ) : showApiKey ? (
                          <EyeOff className="w-4 h-4 text-slate-400" />
                        ) : (
                          <Eye className="w-4 h-4 text-slate-400" />
                        )}
                      </button>
                    )}
                  </div>
                  <button
                    onClick={handleValidateKey}
                    disabled={!apiKey.trim() || apiKey.startsWith("****") || validateApiKey.isPending}
                    className={clsx(
                      "px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg transition-colors",
                      "disabled:opacity-50 disabled:cursor-not-allowed"
                    )}
                  >
                    {validateApiKey.isPending ? (
                      <Loader2 className="w-5 h-5 animate-spin" />
                    ) : (
                      "Validate"
                    )}
                  </button>
                </div>

                {/* Validation status */}
                {validateApiKey.isSuccess && (
                  <div className="flex items-center gap-2 mt-2 text-green-400 text-sm">
                    <CheckCircle className="w-4 h-4" />
                    API key is valid
                  </div>
                )}
                {validateApiKey.isError && (
                  <div className="flex items-center gap-2 mt-2 text-red-400 text-sm">
                    <AlertCircle className="w-4 h-4" />
                    Invalid API key
                  </div>
                )}

                <p className="text-xs text-slate-500 mt-2">
                  Get your API key from{" "}
                  <a
                    href="https://aistudio.google.com/apikey"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-400 hover:underline"
                  >
                    Google AI Studio
                  </a>
                </p>
              </div>

              {/* Remove API Key */}
              {apiKey && (
                <div className="flex items-center justify-between border-t border-slate-600 pt-4">
                  <div>
                    <p className="text-sm font-medium text-red-400">Remove API Key</p>
                    <p className="text-xs text-slate-500">Clear your stored API key</p>
                  </div>
                  {showRemoveKeyConfirm ? (
                    <div className="flex gap-2">
                      <button
                        onClick={() => setShowRemoveKeyConfirm(false)}
                        className="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-300 transition-colors"
                      >
                        Cancel
                      </button>
                      <button
                        onClick={handleRemoveApiKey}
                        disabled={clearApiKey.isPending}
                        className="px-3 py-1.5 text-sm bg-red-600 hover:bg-red-500 rounded-lg transition-colors"
                      >
                        {clearApiKey.isPending ? (
                          <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                          "Confirm"
                        )}
                      </button>
                    </div>
                  ) : (
                    <button
                      onClick={() => setShowRemoveKeyConfirm(true)}
                      className="px-3 py-1.5 text-sm bg-red-600/20 border border-red-500/50 text-red-400 hover:bg-red-600/30 rounded-lg transition-colors"
                    >
                      Remove
                    </button>
                  )}
                </div>
              )}

              <div className="bg-slate-700/50 rounded-lg p-4 text-sm">
                <h4 className="font-medium mb-2">About BYOK (Bring Your Own Key)</h4>
                <p className="text-slate-400">
                  RECALL.OS uses the Gemini API for document processing and question
                  answering. Your API key is stored locally on your device and is
                  never sent to our servers.
                </p>
              </div>
            </div>
          ) : activeTab === "watch" ? (
            <div className="space-y-4">
              {/* Auto-import toggle */}
              <div className="flex items-center justify-between">
                <div>
                  <label className="block text-sm font-medium">
                    Auto-Import Files
                  </label>
                  <p className="text-xs text-slate-500">
                    Automatically index new files in synced folders
                  </p>
                </div>
                <button
                  onClick={() => toggleAutoIngest.mutate(!watcherStatus?.auto_ingest_enabled)}
                  className={clsx(
                    "relative w-12 h-6 rounded-full transition-colors",
                    watcherStatus?.auto_ingest_enabled
                      ? "bg-blue-600"
                      : "bg-slate-600"
                  )}
                >
                  <span
                    className={clsx(
                      "absolute left-0 top-1 w-4 h-4 rounded-full bg-white transition-transform",
                      watcherStatus?.auto_ingest_enabled
                        ? "translate-x-7"
                        : "translate-x-1"
                    )}
                  />
                </button>
              </div>

              {/* Sync status */}
              <div className="flex items-center gap-2 text-sm">
                <span
                  className={clsx(
                    "w-2 h-2 rounded-full",
                    watcherStatus?.is_running ? "bg-green-500" : "bg-slate-500"
                  )}
                />
                <span className="text-slate-400">
                  {watcherStatus?.is_running
                    ? `Syncing ${watcherStatus.watched_folders.length} folder(s)`
                    : "Sync paused"}
                </span>
              </div>

              {/* Add folder button */}
              <button
                onClick={handleAddPendingFolder}
                className="flex items-center gap-2 w-full px-4 py-3 bg-slate-700 hover:bg-slate-600 rounded-lg transition-colors"
              >
                <Plus className="w-4 h-4" />
                <span>Add Folder</span>
              </button>

              {/* Synced folders list */}
              <div className="space-y-2">
                <label className="block text-sm font-medium">
                  Synced Folders
                </label>
                {watcherStatus?.watched_folders.length === 0 && pendingFolders.length === 0 ? (
                  <p className="text-sm text-slate-500 py-4 text-center">
                    No folders being synced. Add a folder to start auto-indexing.
                  </p>
                ) : (
                  <div className="space-y-2 max-h-48 overflow-y-auto">
                    {/* Pending folders (not yet saved) */}
                    {pendingFolders.map((folder) => (
                      <div
                        key={`pending-${folder}`}
                        className="flex items-center justify-between bg-slate-700/50 border border-dashed border-slate-500 rounded-lg px-3 py-2"
                      >
                        <div className="flex items-center gap-2 min-w-0">
                          <FolderOpen className="w-4 h-4 text-yellow-400 flex-shrink-0" />
                          <span className="text-sm truncate" title={folder}>
                            {folder}
                          </span>
                          <span className="flex items-center gap-1 px-1.5 py-0.5 bg-yellow-500/20 text-yellow-400 text-xs rounded">
                            <Clock className="w-3 h-3" />
                            Pending
                          </span>
                        </div>
                        <button
                          onClick={() => handleRemovePendingFolder(folder)}
                          className="p-1 hover:bg-slate-600 rounded transition-colors flex-shrink-0"
                          title="Remove folder"
                        >
                          <Trash2 className="w-4 h-4 text-red-400" />
                        </button>
                      </div>
                    ))}
                    {/* Already synced folders */}
                    {watcherStatus?.watched_folders.map((folder) => (
                      <div
                        key={folder}
                        className="flex items-center justify-between bg-slate-700 rounded-lg px-3 py-2"
                      >
                        <div className="flex items-center gap-2 min-w-0">
                          <FolderOpen className="w-4 h-4 text-blue-400 flex-shrink-0" />
                          <span className="text-sm truncate" title={folder}>
                            {folder}
                          </span>
                        </div>
                        <button
                          onClick={() => removeWatchedFolder.mutate(folder)}
                          className="p-1 hover:bg-slate-600 rounded transition-colors flex-shrink-0"
                          title="Remove folder"
                        >
                          <Trash2 className="w-4 h-4 text-red-400" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              <div className="bg-slate-700/50 rounded-lg p-4 text-sm">
                <h4 className="font-medium mb-2">How it works</h4>
                <p className="text-slate-400">
                  When auto-import is enabled, RECALL.OS monitors your selected folders
                  for new or modified files and automatically adds them to your
                  knowledge base.
                </p>
              </div>
            </div>
          ) : activeTab === "capture" ? (
            <ScreenCaptureSettings />
          ) : (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">
                  Chunk Size (tokens)
                </label>
                <input
                  type="number"
                  value={localSettings.chunk_size || 512}
                  onChange={(e) =>
                    setLocalSettings((prev) => ({
                      ...prev,
                      chunk_size: parseInt(e.target.value) || 512,
                    }))
                  }
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 focus:outline-none focus:border-blue-500"
                />
                <p className="text-xs text-slate-500 mt-1">
                  Recommended: 512 tokens for balanced retrieval
                </p>
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">
                  Chunk Overlap (tokens)
                </label>
                <input
                  type="number"
                  value={localSettings.chunk_overlap || 50}
                  onChange={(e) =>
                    setLocalSettings((prev) => ({
                      ...prev,
                      chunk_overlap: parseInt(e.target.value) || 50,
                    }))
                  }
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 focus:outline-none focus:border-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">
                  Max Context Chunks
                </label>
                <input
                  type="number"
                  value={localSettings.max_context_chunks || 20}
                  onChange={(e) =>
                    setLocalSettings((prev) => ({
                      ...prev,
                      max_context_chunks: parseInt(e.target.value) || 20,
                    }))
                  }
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 focus:outline-none focus:border-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">
                  Video Segment Duration (seconds)
                </label>
                <input
                  type="number"
                  value={localSettings.video_segment_duration || 300}
                  onChange={(e) =>
                    setLocalSettings((prev) => ({
                      ...prev,
                      video_segment_duration: parseInt(e.target.value) || 300,
                    }))
                  }
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 focus:outline-none focus:border-blue-500"
                />
                <p className="text-xs text-slate-500 mt-1">
                  Videos are processed in segments. Recommended: 300 seconds (5 minutes)
                </p>
              </div>

              {/* Danger Zone */}
              <div className="border-t border-slate-600 pt-4 mt-4">
                <h4 className="text-sm font-medium text-red-400 mb-3">Danger Zone</h4>
                <div className="flex items-center justify-between bg-red-900/10 border border-red-500/30 rounded-lg p-3">
                  <div>
                    <p className="text-sm font-medium">Reset Database</p>
                    <p className="text-xs text-slate-500">
                      Delete all documents, chunks, and conversations
                    </p>
                  </div>
                  {showResetConfirm ? (
                    <div className="flex gap-2">
                      <button
                        onClick={() => setShowResetConfirm(false)}
                        className="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-300 transition-colors"
                      >
                        Cancel
                      </button>
                      <button
                        onClick={handleResetDatabase}
                        disabled={resetDatabase.isPending}
                        className="px-3 py-1.5 text-sm bg-red-600 hover:bg-red-500 rounded-lg transition-colors"
                      >
                        {resetDatabase.isPending ? (
                          <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                          "Confirm Reset"
                        )}
                      </button>
                    </div>
                  ) : (
                    <button
                      onClick={() => setShowResetConfirm(true)}
                      className="px-3 py-1.5 text-sm bg-red-600/20 border border-red-500/50 text-red-400 hover:bg-red-600/30 rounded-lg transition-colors"
                    >
                      Reset Database
                    </button>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-3 p-4 border-t border-slate-700 flex-shrink-0">
          <button
            onClick={onClose}
            className="px-4 py-2 text-slate-400 hover:text-slate-300 transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleSaveSettings}
            disabled={updateSettings.isPending}
            className={clsx(
              "px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg transition-colors",
              "disabled:opacity-50 disabled:cursor-not-allowed"
            )}
          >
            {updateSettings.isPending ? (
              <Loader2 className="w-5 h-5 animate-spin" />
            ) : (
              "Save Changes"
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
