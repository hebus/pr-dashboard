import type { Provider } from "../types";

// lucide-react ships no brand icons (logos were removed from the library),
// so the two marks live here as inline SVG paths using currentColor.

interface Props {
  provider: Provider;
  size?: number;
  className?: string;
}

export function ProviderIcon({ provider, size = 14, className }: Props) {
  const common = {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "currentColor",
    className,
    "aria-hidden": true,
  } as const;

  if (provider === "gitlab") {
    return (
      <svg {...common}>
        <title>GitLab</title>
        <path d="M12 22.1 16.42 8.5H7.58L12 22.1Z" />
        <path d="M12 22.1 7.58 8.5H1.39L12 22.1Z" opacity=".7" />
        <path d="M1.39 8.5 0.05 12.63a.92.92 0 0 0 .33 1.03L12 22.1 1.39 8.5Z" opacity=".5" />
        <path d="M1.39 8.5h6.19L4.92 0.32a.46.46 0 0 0-.88 0L1.39 8.5Z" opacity=".85" />
        <path d="M12 22.1 16.42 8.5h6.19L12 22.1Z" opacity=".7" />
        <path d="M22.61 8.5l1.34 4.13a.92.92 0 0 1-.33 1.03L12 22.1 22.61 8.5Z" opacity=".5" />
        <path d="M22.61 8.5h-6.19l2.66-8.18a.46.46 0 0 1 .88 0L22.61 8.5Z" opacity=".85" />
      </svg>
    );
  }

  return (
    <svg {...common}>
      <title>GitHub</title>
      <path d="M12 .3a12 12 0 0 0-3.79 23.4c.6.1.82-.26.82-.58v-2.23c-3.34.72-4.04-1.6-4.04-1.6-.55-1.4-1.34-1.77-1.34-1.77-1.09-.74.08-.73.08-.73 1.2.09 1.84 1.24 1.84 1.24 1.07 1.84 2.81 1.31 3.5 1 .1-.78.42-1.31.76-1.61-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.14-.3-.54-1.52.1-3.18 0 0 1-.32 3.3 1.23a11.5 11.5 0 0 1 6 0c2.28-1.55 3.29-1.23 3.29-1.23.65 1.66.24 2.88.12 3.18a4.65 4.65 0 0 1 1.23 3.22c0 4.61-2.8 5.63-5.48 5.92.43.37.81 1.1.81 2.22v3.29c0 .32.22.69.83.57A12 12 0 0 0 12 .3Z" />
    </svg>
  );
}
