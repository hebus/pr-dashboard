import { Component, type ReactNode } from "react";

interface Props  { children: ReactNode; }
interface State  { error: Error | null; }

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex items-center justify-center h-screen bg-[var(--c-bg)] text-[var(--c-text)] p-8">
          <div className="max-w-lg text-center">
            <div className="text-[var(--c-red)] text-5xl mb-4">!</div>
            <h1 className="text-xl font-semibold mb-2">Something went wrong</h1>
            <pre className="text-sm text-[var(--c-text-muted)] bg-[var(--c-bg-subtle)] border border-[var(--c-border)] rounded p-4 text-left overflow-auto">
              {this.state.error.message}
            </pre>
            <button
              onClick={() => this.setState({ error: null })}
              className="mt-4 px-4 py-2 bg-[var(--c-green-btn)] hover:bg-[var(--c-green-btn-hover)] text-white rounded text-sm transition-colors"
            >
              Retry
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
