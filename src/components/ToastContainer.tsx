import { useState, useEffect } from "react";
import { X, GitPullRequest, GitMerge } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { Toast } from "../hooks/useToast";

function ToastItem({ toast, onDismiss }: { toast: Toast; onDismiss: () => void }) {
  const [visible, setVisible] = useState(false);
  const isNew = toast.type === "new_pr";

  useEffect(() => {
    const id = requestAnimationFrame(() => setVisible(true));
    return () => cancelAnimationFrame(id);
  }, []);

  function handleClick() {
    if (toast.url) invoke("open_url", { url: toast.url });
  }

  return (
    <div
      onClick={handleClick}
      style={{
        transform: visible ? "translateX(0)" : "translateX(calc(100% + 1rem))",
        opacity: visible ? 1 : 0,
        transition: "transform 0.25s ease-out, opacity 0.25s ease-out",
      }}
      className="w-72 bg-[var(--c-bg-subtle)] border border-[var(--c-border)] rounded-lg shadow-xl overflow-hidden cursor-pointer hover:bg-[var(--c-bg-hover)] transition-colors"
    >
      <div className="p-3 flex items-start gap-2.5">
        <div
          className={`mt-0.5 shrink-0 ${
            isNew ? "text-[var(--c-accent)]" : "text-[var(--c-green)]"
          }`}
        >
          {isNew ? <GitPullRequest size={14} /> : <GitMerge size={14} />}
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-xs font-semibold text-[var(--c-text)]">
            {isNew ? "New PR" : "PR merged"}
            <span className="font-normal text-[var(--c-text-muted)]">
              {" "}· {toast.repo}
            </span>
          </p>
          <p className="text-xs text-[var(--c-text-muted)] mt-0.5 truncate">
            #{toast.prNumber} — {toast.prTitle}
          </p>
        </div>
        <button
          onClick={(e) => {
            e.stopPropagation();
            onDismiss();
          }}
          className="shrink-0 text-[var(--c-text-muted)] hover:text-[var(--c-text)] transition-colors mt-0.5"
        >
          <X size={12} />
        </button>
      </div>
      <div className="h-0.5 bg-[var(--c-bg-inset)]">
        <div
          className={`h-full ${isNew ? "bg-[var(--c-accent)]" : "bg-[var(--c-green)]"}`}
          style={{ animation: "toastProgress 5s linear forwards" }}
        />
      </div>
    </div>
  );
}

interface Props {
  toasts: Toast[];
  onDismiss: (id: string) => void;
}

export function ToastContainer({ toasts, onDismiss }: Props) {
  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 items-end pointer-events-none">
      {toasts.map((t) => (
        <div key={t.id} className="pointer-events-auto">
          <ToastItem toast={t} onDismiss={() => onDismiss(t.id)} />
        </div>
      ))}
    </div>
  );
}
