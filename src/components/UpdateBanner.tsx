import { Download, RefreshCw, X } from "lucide-react";
import type { UpdaterStatus } from "../hooks/useUpdater";

interface Props {
  status: UpdaterStatus;
  version: string | null;
  progress: number | null;
  error: string | null;
  onInstall: () => void;
  onDismiss: () => void;
}

export function UpdateBanner({ status, version, progress, error, onInstall, onDismiss }: Props) {
  if (status === "idle") return null;

  const installing = status === "installing";

  return (
    <div className="flex items-center justify-between gap-3 px-4 py-2 border-b border-[var(--c-indigo-border)] bg-[var(--c-indigo-bg)] text-[var(--c-indigo)] text-xs shrink-0">
      <div className="flex items-center gap-2 min-w-0">
        {installing ? (
          <RefreshCw size={13} className="animate-spin shrink-0" />
        ) : (
          <Download size={13} className="shrink-0" />
        )}
        <span className="truncate">
          {installing
            ? `Downloading version ${version}${progress !== null ? ` — ${progress}%` : "…"}`
            : status === "error"
              ? `Update to ${version} failed: ${error}`
              : `Version ${version} is available.`}
        </span>
      </div>

      <div className="flex items-center gap-2 shrink-0">
        {!installing && (
          <button
            onClick={onInstall}
            className="px-2.5 py-1 rounded bg-[var(--c-indigo)] text-white font-medium hover:opacity-90 transition-opacity"
          >
            {status === "error" ? "Retry" : "Install and restart"}
          </button>
        )}
        {!installing && (
          <button
            onClick={onDismiss}
            title="Later"
            className="p-1 rounded hover:bg-[var(--c-indigo-border)] transition-colors"
          >
            <X size={13} />
          </button>
        )}
      </div>
    </div>
  );
}
