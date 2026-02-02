import { useState, useEffect } from "react";
import {
  Camera,
  Monitor,
  AppWindow,
  Play,
  Pause,
  Loader2,
  Plus,
  Trash2,
  RefreshCw,
  Clock,
  Shield,
} from "lucide-react";
import {
  useCaptureStatus,
  useStartCapture,
  useStopCapture,
  useCaptureNow,
  usePauseCapture,
  useResumeCapture,
  useRunningApps,
  useUpdateCaptureSettings,
  useCleanupCaptures,
} from "../hooks/useCapture";
import { useSettings } from "../hooks/useSettings";
import clsx from "clsx";

export default function ScreenCaptureSettings() {
  const { data: settings } = useSettings();
  const { data: captureStatus } = useCaptureStatus();
  const { data: runningApps, refetch: refetchApps } = useRunningApps();

  const startCapture = useStartCapture();
  const stopCapture = useStopCapture();
  const captureNow = useCaptureNow();
  const pauseCapture = usePauseCapture();
  const resumeCapture = useResumeCapture();
  const updateSettings = useUpdateCaptureSettings();
  const cleanupCaptures = useCleanupCaptures();

  // Local state for form
  const [enabled, setEnabled] = useState(false);
  const [intervalSecs, setIntervalSecs] = useState(60);
  const [mode, setMode] = useState<"active_window" | "full_screen">("active_window");
  const [filterMode, setFilterMode] = useState<"none" | "whitelist" | "blacklist">("none");
  const [appList, setAppList] = useState<string[]>([]);
  const [retentionDays, setRetentionDays] = useState(7);
  const [hotkey, setHotkey] = useState("Ctrl+Shift+S");
  const [showAppPicker, setShowAppPicker] = useState(false);

  // Sync settings to local state
  useEffect(() => {
    if (settings) {
      setEnabled(settings.screen_capture_enabled);
      setIntervalSecs(settings.capture_interval_secs);
      setMode(settings.capture_mode);
      setFilterMode(settings.capture_app_filter);
      setAppList(settings.capture_app_list);
      setRetentionDays(settings.capture_retention_days);
      setHotkey(settings.capture_hotkey);
    }
  }, [settings]);

  const handleToggleEnabled = async () => {
    if (enabled) {
      await stopCapture.mutateAsync();
      setEnabled(false);
    } else {
      await startCapture.mutateAsync();
      setEnabled(true);
    }
  };

  const handleSaveSettings = async () => {
    await updateSettings.mutateAsync({
      enabled,
      interval_secs: intervalSecs,
      mode,
      filter_mode: filterMode,
      app_list: appList,
      retention_days: retentionDays,
      hotkey,
    });
  };

  const handleCaptureNow = async () => {
    try {
      await captureNow.mutateAsync();
    } catch (error) {
      console.error("Capture failed:", error);
    }
  };

  const handleAddApp = (appName: string) => {
    if (!appList.includes(appName)) {
      setAppList([...appList, appName]);
    }
    setShowAppPicker(false);
  };

  const handleRemoveApp = (appName: string) => {
    setAppList(appList.filter((a) => a !== appName));
  };

  const formatLastCapture = (dateStr: string | null) => {
    if (!dateStr) return "Never";
    const date = new Date(dateStr);
    return date.toLocaleTimeString();
  };

  const intervalOptions = [
    { value: 30, label: "30 seconds" },
    { value: 60, label: "1 minute" },
    { value: 120, label: "2 minutes" },
    { value: 180, label: "3 minutes" },
    { value: 300, label: "5 minutes" },
  ];

  const retentionOptions = [
    { value: 1, label: "1 day" },
    { value: 3, label: "3 days" },
    { value: 7, label: "1 week" },
    { value: 14, label: "2 weeks" },
    { value: 30, label: "1 month" },
  ];

  return (
    <div className="space-y-6">
      {/* Status Card */}
      <div className="bg-slate-700/50 rounded-lg p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div
              className={clsx(
                "w-3 h-3 rounded-full",
                captureStatus?.scheduler_running
                  ? captureStatus?.paused
                    ? "bg-yellow-500"
                    : "bg-green-500 animate-pulse"
                  : "bg-slate-500"
              )}
            />
            <div>
              <p className="font-medium">
                {captureStatus?.scheduler_running
                  ? captureStatus?.paused
                    ? "Paused"
                    : "Capturing"
                  : "Stopped"}
              </p>
              <p className="text-xs text-slate-400">
                {captureStatus?.capture_count || 0} captures this session
                {captureStatus?.last_capture && (
                  <> - Last: {formatLastCapture(captureStatus.last_capture)}</>
                )}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            {captureStatus?.scheduler_running && (
              <button
                onClick={() =>
                  captureStatus?.paused
                    ? resumeCapture.mutate()
                    : pauseCapture.mutate()
                }
                className="p-2 hover:bg-slate-600 rounded-lg transition-colors"
                title={captureStatus?.paused ? "Resume" : "Pause"}
              >
                {captureStatus?.paused ? (
                  <Play className="w-4 h-4" />
                ) : (
                  <Pause className="w-4 h-4" />
                )}
              </button>
            )}
            <button
              onClick={handleCaptureNow}
              disabled={captureNow.isPending}
              className={clsx(
                "flex items-center gap-2 px-3 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg transition-colors",
                "disabled:opacity-50 disabled:cursor-not-allowed"
              )}
            >
              {captureNow.isPending ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Camera className="w-4 h-4" />
              )}
              <span>Capture Now</span>
            </button>
          </div>
        </div>
      </div>

      {/* Enable Toggle */}
      <div className="flex items-center justify-between">
        <div>
          <label className="block text-sm font-medium">
            Periodic Screen Capture
          </label>
          <p className="text-xs text-slate-500">
            Automatically capture screenshots at regular intervals
          </p>
        </div>
        <button
          onClick={handleToggleEnabled}
          disabled={startCapture.isPending || stopCapture.isPending}
          className={clsx(
            "relative w-12 h-6 rounded-full transition-colors",
            enabled ? "bg-blue-600" : "bg-slate-600"
          )}
        >
          <span
            className={clsx(
              "absolute left-0 top-1 w-4 h-4 rounded-full bg-white transition-transform",
              enabled ? "translate-x-7" : "translate-x-1"
            )}
          />
        </button>
      </div>

      {/* Capture Mode */}
      <div>
        <label className="block text-sm font-medium mb-2">Capture Mode</label>
        <div className="grid grid-cols-2 gap-2">
          <button
            onClick={() => setMode("active_window")}
            className={clsx(
              "flex items-center gap-2 p-3 rounded-lg border transition-colors",
              mode === "active_window"
                ? "border-blue-500 bg-blue-500/10"
                : "border-slate-600 hover:border-slate-500"
            )}
          >
            <AppWindow className="w-5 h-5" />
            <div className="text-left">
              <p className="text-sm font-medium">Active Window</p>
              <p className="text-xs text-slate-400">Current focused window only</p>
            </div>
          </button>
          <button
            onClick={() => setMode("full_screen")}
            className={clsx(
              "flex items-center gap-2 p-3 rounded-lg border transition-colors",
              mode === "full_screen"
                ? "border-blue-500 bg-blue-500/10"
                : "border-slate-600 hover:border-slate-500"
            )}
          >
            <Monitor className="w-5 h-5" />
            <div className="text-left">
              <p className="text-sm font-medium">Full Screen</p>
              <p className="text-xs text-slate-400">Entire primary display</p>
            </div>
          </button>
        </div>
      </div>

      {/* Capture Interval */}
      <div>
        <label className="block text-sm font-medium mb-2">
          <Clock className="w-4 h-4 inline mr-2" />
          Capture Interval
        </label>
        <select
          value={intervalSecs}
          onChange={(e) => setIntervalSecs(Number(e.target.value))}
          className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 focus:outline-none focus:border-blue-500"
        >
          {intervalOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>

      {/* App Filter */}
      <div>
        <label className="block text-sm font-medium mb-2">
          <Shield className="w-4 h-4 inline mr-2" />
          Application Filter
        </label>
        <select
          value={filterMode}
          onChange={(e) => setFilterMode(e.target.value as typeof filterMode)}
          className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 focus:outline-none focus:border-blue-500"
        >
          <option value="none">No filter (capture all apps)</option>
          <option value="whitelist">Whitelist (only capture listed apps)</option>
          <option value="blacklist">Blacklist (skip listed apps)</option>
        </select>

        {filterMode !== "none" && (
          <div className="mt-3 space-y-2">
            <div className="flex items-center justify-between">
              <label className="text-sm text-slate-400">
                {filterMode === "whitelist" ? "Only capture:" : "Skip:"}
              </label>
              <button
                onClick={() => {
                  refetchApps();
                  setShowAppPicker(true);
                }}
                className="flex items-center gap-1 text-sm text-blue-400 hover:text-blue-300"
              >
                <Plus className="w-4 h-4" />
                Add App
              </button>
            </div>

            {appList.length === 0 ? (
              <p className="text-sm text-slate-500 py-2">
                No apps in {filterMode}. Click "Add App" to add one.
              </p>
            ) : (
              <div className="space-y-1 max-h-32 overflow-y-auto">
                {appList.map((app) => (
                  <div
                    key={app}
                    className="flex items-center justify-between bg-slate-700 rounded px-3 py-2"
                  >
                    <span className="text-sm truncate">{app}</span>
                    <button
                      onClick={() => handleRemoveApp(app)}
                      className="p-1 hover:bg-slate-600 rounded"
                    >
                      <Trash2 className="w-3 h-3 text-red-400" />
                    </button>
                  </div>
                ))}
              </div>
            )}

            {/* App Picker Modal */}
            {showAppPicker && (
              <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
                <div className="bg-slate-800 rounded-lg w-96 max-h-96 overflow-hidden">
                  <div className="flex items-center justify-between p-3 border-b border-slate-700">
                    <h3 className="font-medium">Running Applications</h3>
                    <button
                      onClick={() => setShowAppPicker(false)}
                      className="p-1 hover:bg-slate-700 rounded"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                  <div className="p-2 max-h-72 overflow-y-auto">
                    {runningApps?.map((app) => (
                      <button
                        key={app.process_name}
                        onClick={() => handleAddApp(app.process_name)}
                        disabled={appList.includes(app.process_name)}
                        className={clsx(
                          "w-full text-left px-3 py-2 rounded hover:bg-slate-700 transition-colors",
                          appList.includes(app.process_name) && "opacity-50"
                        )}
                      >
                        <p className="text-sm font-medium">{app.process_name}</p>
                        <p className="text-xs text-slate-400 truncate">
                          {app.window_title}
                        </p>
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Retention */}
      <div>
        <label className="block text-sm font-medium mb-2">
          Screenshot Retention
        </label>
        <div className="flex items-center gap-4">
          <select
            value={retentionDays}
            onChange={(e) => setRetentionDays(Number(e.target.value))}
            className="flex-1 bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 focus:outline-none focus:border-blue-500"
          >
            {retentionOptions.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
          <button
            onClick={() => cleanupCaptures.mutate()}
            disabled={cleanupCaptures.isPending}
            className="flex items-center gap-1 px-3 py-2 text-sm text-slate-400 hover:text-slate-300 hover:bg-slate-700 rounded-lg transition-colors"
            title="Clean up old captures now"
          >
            {cleanupCaptures.isPending ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            Clean Now
          </button>
        </div>
        <p className="text-xs text-slate-500 mt-1">
          Screenshots older than this will be automatically deleted
        </p>
      </div>

      {/* Hotkey */}
      <div>
        <label className="block text-sm font-medium mb-2">
          Quick Capture Hotkey
        </label>
        <input
          type="text"
          value={hotkey}
          onChange={(e) => setHotkey(e.target.value)}
          placeholder="e.g., Ctrl+Shift+S"
          className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 focus:outline-none focus:border-blue-500"
        />
        <p className="text-xs text-slate-500 mt-1">
          Press this key combination to capture instantly from anywhere
        </p>
      </div>

      {/* Privacy Note */}
      <div className="bg-slate-700/50 rounded-lg p-4">
        <div className="flex items-start gap-3">
          <Shield className="w-5 h-5 text-blue-400 flex-shrink-0 mt-0.5" />
          <div>
            <h4 className="font-medium text-sm">Privacy Protection</h4>
            <p className="text-xs text-slate-400 mt-1">
              Windows with sensitive content (passwords, banking, etc.) are
              automatically skipped. All screenshots are stored locally and
              processed on your device.
            </p>
          </div>
        </div>
      </div>

      {/* Save Button */}
      <div className="flex justify-end">
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
            "Save Settings"
          )}
        </button>
      </div>
    </div>
  );
}
