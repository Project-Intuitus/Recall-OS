import { useState } from "react";
import { X, Key, Loader2, CheckCircle, AlertCircle, Shield, ExternalLink } from "lucide-react";
import { useLicenseStatus, useActivateLicense, useDeactivateLicense, useActivateTestLicense } from "../hooks/useLicense";
import clsx from "clsx";

interface LicenseModalProps {
  onClose: () => void;
}

export default function LicenseModal({ onClose }: LicenseModalProps) {
  const { data: licenseStatus, isLoading } = useLicenseStatus();
  const activateLicense = useActivateLicense();
  const deactivateLicense = useDeactivateLicense();
  const activateTestLicense = useActivateTestLicense();

  const [licenseKey, setLicenseKey] = useState("");
  const [showDeactivateConfirm, setShowDeactivateConfirm] = useState(false);

  // Check if running in dev mode (Vite sets this)
  const isDev = import.meta.env.DEV;

  const handleActivate = async () => {
    if (!licenseKey.trim()) return;

    try {
      await activateLicense.mutateAsync(licenseKey.trim());
      setLicenseKey("");
    } catch (error) {
      console.error("License activation failed:", error);
    }
  };

  const handleDeactivate = async () => {
    try {
      await deactivateLicense.mutateAsync();
      setShowDeactivateConfirm(false);
    } catch (error) {
      console.error("License deactivation failed:", error);
    }
  };

  const handleTestActivate = async () => {
    try {
      await activateTestLicense.mutateAsync();
    } catch (error) {
      console.error("Test license activation failed:", error);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 animate-fade-in p-4">
      <div className="glass-elevated rounded-2xl w-full max-w-md shadow-2xl animate-slide-in border border-slate-700/50">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-slate-700/50">
          <div className="flex items-center gap-2">
            <Shield className="w-5 h-5 text-cyan-400" />
            <h2 className="text-lg font-semibold text-white">License</h2>
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-slate-700/50 rounded-lg transition-colors text-slate-400 hover:text-white"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-4 space-y-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="w-6 h-6 animate-spin" />
            </div>
          ) : licenseStatus?.is_valid ? (
            // Licensed state
            <div className="space-y-4">
              <div className="bg-green-500/10 border border-green-500/30 rounded-lg p-4">
                <div className="flex items-center gap-2 text-green-400">
                  <CheckCircle className="w-5 h-5" />
                  <span className="font-medium">License Active</span>
                </div>
                <p className="text-sm text-slate-400 mt-2">
                  Your license is valid and all features are unlocked.
                </p>
              </div>

              <div className="bg-slate-700/50 rounded-lg p-3 space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-slate-400">License Key</span>
                  <span className="font-mono">{licenseStatus.license_key}</span>
                </div>
                {licenseStatus.activated_at && (
                  <div className="flex justify-between text-sm">
                    <span className="text-slate-400">Activated</span>
                    <span>
                      {new Date(licenseStatus.activated_at).toLocaleDateString()}
                    </span>
                  </div>
                )}
                <div className="flex justify-between text-sm">
                  <span className="text-slate-400">Tier</span>
                  <span className="capitalize text-green-400">
                    {licenseStatus.tier}
                  </span>
                </div>
              </div>

              {/* Deactivate option */}
              <div className="pt-4 border-t border-slate-700">
                {showDeactivateConfirm ? (
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-slate-400">
                      Remove license from this device?
                    </span>
                    <div className="flex gap-2">
                      <button
                        onClick={() => setShowDeactivateConfirm(false)}
                        className="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-300 transition-colors"
                      >
                        Cancel
                      </button>
                      <button
                        onClick={handleDeactivate}
                        disabled={deactivateLicense.isPending}
                        className="px-3 py-1.5 text-sm bg-red-600 hover:bg-red-500 rounded-lg transition-colors"
                      >
                        {deactivateLicense.isPending ? (
                          <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                          "Confirm"
                        )}
                      </button>
                    </div>
                  </div>
                ) : (
                  <button
                    onClick={() => setShowDeactivateConfirm(true)}
                    className="text-sm text-slate-400 hover:text-red-400 transition-colors"
                  >
                    Deactivate License
                  </button>
                )}
              </div>
            </div>
          ) : (
            // Unlicensed state
            <div className="space-y-4">
              <div className="bg-amber-500/10 border border-amber-500/30 rounded-lg p-4">
                <div className="flex items-center gap-2 text-amber-400">
                  <Key className="w-5 h-5" />
                  <span className="font-medium">Trial Mode</span>
                </div>
                <p className="text-sm text-slate-400 mt-2">
                  You&apos;re using RECALL.OS in trial mode. Purchase a license
                  to unlock unlimited documents.
                </p>
                {licenseStatus && licenseStatus.documents_used !== null && licenseStatus.documents_limit !== null && (
                  <div className="mt-3 pt-3 border-t border-amber-500/20">
                    <div className="flex justify-between text-sm mb-1">
                      <span className="text-slate-400">Documents</span>
                      <span className={clsx(
                        (licenseStatus.documents_used ?? 0) >= (licenseStatus.documents_limit ?? 1)
                          ? "text-red-400"
                          : "text-amber-400"
                      )}>
                        {licenseStatus.documents_used} / {licenseStatus.documents_limit}
                      </span>
                    </div>
                    <div className="w-full bg-slate-700 rounded-full h-2">
                      <div
                        className={clsx(
                          "h-2 rounded-full transition-all",
                          (licenseStatus.documents_used ?? 0) >= (licenseStatus.documents_limit ?? 1)
                            ? "bg-red-500"
                            : "bg-amber-500"
                        )}
                        style={{
                          width: `${Math.min(100, ((licenseStatus.documents_used ?? 0) / (licenseStatus.documents_limit ?? 1)) * 100)}%`
                        }}
                      />
                    </div>
                    {(licenseStatus.documents_used ?? 0) >= (licenseStatus.documents_limit ?? 1) && (
                      <p className="text-xs text-red-400 mt-2">
                        Trial limit reached. Upgrade to add more documents.
                      </p>
                    )}
                  </div>
                )}
              </div>

              {/* License key input */}
              <div>
                <label className="block text-sm font-medium mb-2">
                  Enter License Key
                </label>
                <input
                  type="text"
                  value={licenseKey}
                  onChange={(e) => setLicenseKey(e.target.value)}
                  placeholder="Paste your license key here"
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 font-mono text-sm focus:outline-none focus:border-blue-500"
                />
              </div>

              {/* Activation button */}
              <button
                onClick={handleActivate}
                disabled={licenseKey.trim().length < 8 || activateLicense.isPending}
                className={clsx(
                  "w-full py-2.5 bg-blue-600 hover:bg-blue-500 rounded-lg transition-colors font-medium",
                  "disabled:opacity-50 disabled:cursor-not-allowed"
                )}
              >
                {activateLicense.isPending ? (
                  <Loader2 className="w-5 h-5 animate-spin mx-auto" />
                ) : (
                  "Activate License"
                )}
              </button>

              {/* Dev-only test license button */}
              {isDev && (
                <button
                  onClick={handleTestActivate}
                  disabled={activateTestLicense.isPending}
                  className="w-full py-2 bg-purple-600 hover:bg-purple-500 rounded-lg transition-colors text-sm"
                >
                  {activateTestLicense.isPending ? (
                    <Loader2 className="w-4 h-4 animate-spin mx-auto" />
                  ) : (
                    "Activate Test License (Dev Only)"
                  )}
                </button>
              )}

              {/* Error message */}
              {activateLicense.isError && (
                <div className="flex items-center gap-2 text-red-400 text-sm">
                  <AlertCircle className="w-4 h-4" />
                  <span>
                    {(activateLicense.error as Error)?.message ||
                      "Activation failed"}
                  </span>
                </div>
              )}

              {/* Purchase link */}
              <div className="text-center pt-2">
                <a
                  href="https://projectintuitus.lemonsqueezy.com/checkout/buy/7f77499e-81dc-4284-a288-eab6aa8d76cb"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 text-sm text-blue-400 hover:underline"
                >
                  Purchase a license
                  <ExternalLink className="w-3 h-3" />
                </a>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-slate-700">
          <button
            onClick={onClose}
            className="w-full py-2 text-slate-400 hover:text-slate-300 transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
