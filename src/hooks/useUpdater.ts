import { useCallback, useEffect, useRef, useState } from "react";
import type { Update } from "@tauri-apps/plugin-updater";

const FIRST_CHECK_DELAY_MS = 5_000;
const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

export type UpdaterStatus = "idle" | "available" | "installing" | "error";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Checks GitHub Releases (via the Tauri updater endpoint) at startup and every
 * few hours. Nothing is downloaded until the user calls `install()`.
 *
 * The plugins are imported lazily so `npm run dev` in a plain browser — where
 * there is no Tauri runtime — never touches them.
 */
export function useUpdater() {
  const [status, setStatus] = useState<UpdaterStatus>("idle");
  const [version, setVersion] = useState<string | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const updateRef = useRef<Update | null>(null);
  const busyRef = useRef(false);

  const checkNow = useCallback(async () => {
    if (!isTauri() || busyRef.current) return;
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (!update) return;
      // A newer release than the one already announced re-opens the banner.
      if (updateRef.current?.version !== update.version) setDismissed(false);
      updateRef.current = update;
      setVersion(update.version);
      setStatus("available");
    } catch (e) {
      // Offline, no release yet, endpoint down… never worth bothering the user:
      // the next scheduled check will try again.
      console.warn("Update check failed:", e);
    }
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    const first = setTimeout(checkNow, FIRST_CHECK_DELAY_MS);
    const interval = setInterval(checkNow, CHECK_INTERVAL_MS);
    return () => {
      clearTimeout(first);
      clearInterval(interval);
    };
  }, [checkNow]);

  const install = useCallback(async () => {
    const update = updateRef.current;
    if (!update || busyRef.current) return;
    busyRef.current = true;
    setStatus("installing");
    setProgress(0);
    setError(null);
    try {
      let total = 0;
      let received = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        else if (event.event === "Progress") {
          received += event.data.chunkLength;
          if (total > 0) setProgress(Math.min(100, Math.round((received / total) * 100)));
        }
      });
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch (e) {
      setError(String(e));
      setStatus("error");
      busyRef.current = false;
    }
  }, []);

  const dismiss = useCallback(() => setDismissed(true), []);

  return { status, version, progress, error, dismissed, install, dismiss };
}
