import { useState } from "react";
import { X, Plus, Trash2, Eye, EyeOff, GitPullRequest, Bell, CheckCircle } from "lucide-react";
import type { Config, Provider, RepositoryConfig } from "../types";
import { generateId } from "../types";
import { ProviderIcon } from "./ProviderIcon";

const DEFAULT_GITLAB_NAMESPACE = "sinequa/rnd";

interface Props {
  config: Config;
  onSave: (config: Config) => Promise<void>;
  onClose: () => void;
  onTestNotification: () => void;
}

function RepoInput({
  repo,
  onUpdate,
  onRemove,
}: {
  repo: RepositoryConfig;
  onUpdate: (r: RepositoryConfig) => void;
  onRemove: () => void;
}) {
  const inputCls = "px-2 py-1.5 bg-[var(--c-bg)] border border-[var(--c-border)] rounded text-sm text-[var(--c-text)] placeholder-[var(--c-text-subtle)] focus:outline-none focus:border-[var(--c-accent)]";
  const isGitlab = repo.provider === "gitlab";

  function toggleProvider() {
    const provider: Provider = isGitlab ? "github" : "gitlab";
    // Pre-fill the namespace when switching to GitLab, but never overwrite
    // something the user already typed.
    const owner =
      provider === "gitlab" && repo.owner.trim() === ""
        ? DEFAULT_GITLAB_NAMESPACE
        : repo.owner;
    onUpdate({ ...repo, provider, owner });
  }

  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={toggleProvider}
        title={`Switch to ${isGitlab ? "GitHub" : "GitLab"}`}
        className="flex items-center gap-1.5 shrink-0 w-[88px] px-2 py-1.5 bg-[var(--c-bg)] border border-[var(--c-border)] rounded text-xs font-medium text-[var(--c-text)] hover:bg-[var(--c-bg-hover)] transition-colors"
      >
        <ProviderIcon provider={repo.provider} size={13} />
        {isGitlab ? "GitLab" : "GitHub"}
      </button>
      <input
        type="text"
        placeholder={isGitlab ? "namespace" : "owner"}
        value={repo.owner}
        onChange={(e) => onUpdate({ ...repo, owner: e.target.value })}
        className={`w-28 ${inputCls}`}
      />
      <span className="text-[var(--c-text-muted)]">/</span>
      <input
        type="text"
        placeholder="repo"
        value={repo.name}
        onChange={(e) => onUpdate({ ...repo, name: e.target.value })}
        className={`flex-1 ${inputCls}`}
      />
      <input
        type="text"
        placeholder="Label (optional)"
        value={repo.label ?? ""}
        onChange={(e) => onUpdate({ ...repo, label: e.target.value || undefined })}
        className={`w-28 ${inputCls}`}
      />
      <button
        onClick={onRemove}
        className="text-[var(--c-text-muted)] hover:text-[var(--c-red)] transition-colors"
      >
        <Trash2 size={14} />
      </button>
    </div>
  );
}

const inputCls = "w-full px-3 py-2 bg-[var(--c-bg)] border border-[var(--c-border)] rounded text-sm text-[var(--c-text)] placeholder-[var(--c-text-subtle)] focus:outline-none focus:border-[var(--c-accent)]";
const labelCls = "text-sm font-medium text-[var(--c-text)]";

function SourceSection({
  provider,
  url,
  token,
  onUrlChange,
  onTokenChange,
}: {
  provider: Provider;
  url: string;
  token: string;
  onUrlChange: (v: string) => void;
  onTokenChange: (v: string) => void;
}) {
  const [showToken, setShowToken] = useState(false);
  const isGitlab = provider === "gitlab";
  const name = isGitlab ? "GitLab" : "GitHub";

  return (
    <div className="flex flex-col gap-3 p-3 border border-[var(--c-border)] rounded-lg">
      <div className="flex items-center gap-2">
        <ProviderIcon provider={provider} size={14} />
        <span className={labelCls}>{name}</span>
      </div>

      <div className="flex flex-col gap-1.5">
        <label className="text-xs text-[var(--c-text-muted)]">
          {isGitlab
            ? "URL de votre instance GitLab."
            : "URL de votre instance GitHub (Enterprise ou github.com)."}
        </label>
        <input
          type="text"
          value={url}
          onChange={(e) => onUrlChange(e.target.value.replace(/\/$/, ""))}
          placeholder={isGitlab ? "https://gitlab.chapsvision.in" : "https://github.sinequa.com"}
          className={`${inputCls} font-mono`}
        />
      </div>

      <div className="flex flex-col gap-1.5">
        <label className="text-xs text-[var(--c-text-muted)]">
          Personal Access Token with{" "}
          <code className="bg-[var(--c-bg-inset)] px-1 rounded">
            {isGitlab ? "read_api" : "repo"}
          </code>{" "}
          scope. Create one at{" "}
          <span className="text-[var(--c-accent)] font-mono text-[11px]">
            {url}
            {isGitlab ? "/-/user_settings/personal_access_tokens" : "/settings/tokens"}
          </span>.
        </label>
        <div className="relative">
          <input
            type={showToken ? "text" : "password"}
            value={token}
            onChange={(e) => onTokenChange(e.target.value)}
            placeholder={isGitlab ? "glpat-..." : "ghp_..."}
            className={`${inputCls} pr-10 font-mono`}
          />
          <button
            onClick={() => setShowToken((v) => !v)}
            className="absolute right-3 top-1/2 -translate-y-1/2 text-[var(--c-text-muted)] hover:text-[var(--c-text)] transition-colors"
          >
            {showToken ? <EyeOff size={14} /> : <Eye size={14} />}
          </button>
        </div>
      </div>
    </div>
  );
}

export function Settings({ config, onSave, onClose, onTestNotification }: Props) {
  const [local, setLocal]         = useState<Config>(config);
  const [saving, setSaving]       = useState(false);
  const [notifSent, setNotifSent] = useState(false);

  async function handleSave() {
    setSaving(true);
    try {
      await onSave(local);
      onClose();
    } finally {
      setSaving(false);
    }
  }

  function addRepo() {
    setLocal((c) => ({
      ...c,
      repositories: [
        ...c.repositories,
        { id: generateId(), provider: "github", owner: "", name: "" },
      ],
    }));
  }

  function updateRepo(id: string, updated: RepositoryConfig) {
    setLocal((c) => ({
      ...c,
      repositories: c.repositories.map((r) => (r.id === id ? updated : r)),
    }));
  }

  function removeRepo(id: string) {
    setLocal((c) => ({
      ...c,
      repositories: c.repositories.filter((r) => r.id !== id),
    }));
  }

  function sendTestNotification() {
    onTestNotification();
    setNotifSent(true);
    setTimeout(() => setNotifSent(false), 3000);
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="w-full max-w-2xl bg-[var(--c-bg-subtle)] border border-[var(--c-border)] rounded-xl shadow-2xl max-h-[90vh] flex flex-col">
        <div className="flex items-center justify-between px-5 py-4 border-b border-[var(--c-border)]">
          <div className="flex items-center gap-2">
            <GitPullRequest size={16} className="text-[var(--c-text)]" />
            <h2 className="font-semibold text-[var(--c-text)]">Settings</h2>
          </div>
          <button
            onClick={onClose}
            className="text-[var(--c-text-muted)] hover:text-[var(--c-text)] transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        <div className="overflow-y-auto flex-1 p-5 flex flex-col gap-5">
          <SourceSection
            provider="github"
            url={local.githubUrl}
            token={local.githubToken}
            onUrlChange={(v) => setLocal((c) => ({ ...c, githubUrl: v }))}
            onTokenChange={(v) => setLocal((c) => ({ ...c, githubToken: v }))}
          />

          <SourceSection
            provider="gitlab"
            url={local.gitlabUrl}
            token={local.gitlabToken}
            onUrlChange={(v) => setLocal((c) => ({ ...c, gitlabUrl: v }))}
            onTokenChange={(v) => setLocal((c) => ({ ...c, gitlabToken: v }))}
          />

          <div className="flex flex-col gap-2">
            <label className={labelCls}>Refresh Interval</label>
            <div className="flex items-center gap-2">
              <input
                type="number"
                min={10}
                max={3600}
                value={local.refreshInterval}
                onChange={(e) =>
                  setLocal((c) => ({
                    ...c,
                    refreshInterval: Math.max(10, Number(e.target.value)),
                  }))
                }
                className="w-24 px-3 py-2 bg-[var(--c-bg)] border border-[var(--c-border)] rounded text-sm text-[var(--c-text)] focus:outline-none focus:border-[var(--c-accent)]"
              />
              <span className="text-sm text-[var(--c-text-muted)]">seconds</span>
            </div>
          </div>

          <div className="flex flex-col gap-2">
            <label className={labelCls}>Notifications</label>
            <div className="flex items-center justify-between px-3 py-2.5 bg-[var(--c-bg)] border border-[var(--c-border)] rounded">
              <span className="text-sm text-[var(--c-text-muted)]">
                OS notifications for new / merged PRs
              </span>
              <button
                onClick={sendTestNotification}
                className="flex items-center gap-1.5 px-3 py-1 text-xs font-medium rounded transition-colors
                  bg-[var(--c-bg-inset)] hover:bg-[var(--c-border)] text-[var(--c-text)]"
              >
                {notifSent
                  ? <><CheckCircle size={12} className="text-[var(--c-green)]" /> Sent</>
                  : <><Bell size={12} /> Test</>
                }
              </button>
            </div>
          </div>

          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <label className={labelCls}>Repositories</label>
              <button
                onClick={addRepo}
                className="flex items-center gap-1 px-2 py-1 text-xs text-[var(--c-accent)] hover:bg-[var(--c-bg-inset)] rounded transition-colors"
              >
                <Plus size={12} />
                Add repo
              </button>
            </div>
            {local.repositories.length === 0 ? (
              <p className="text-xs text-[var(--c-text-subtle)] py-2 text-center">
                No repositories configured. Click "Add repo" to start.
              </p>
            ) : (
              <div className="flex flex-col gap-2">
                {local.repositories.map((repo) => (
                  <RepoInput
                    key={repo.id}
                    repo={repo}
                    onUpdate={(r) => updateRepo(repo.id, r)}
                    onRemove={() => removeRepo(repo.id)}
                  />
                ))}
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center justify-end gap-2 px-5 py-4 border-t border-[var(--c-border)]">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm text-[var(--c-text-muted)] hover:text-[var(--c-text)] transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={saving}
            className="px-4 py-2 bg-[var(--c-green-btn)] hover:bg-[var(--c-green-btn-hover)] disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
          >
            {saving ? "Saving..." : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
