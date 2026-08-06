import { useEffect, useRef } from "react";
import { useQueries } from "@tanstack/react-query";
import type { UseQueryResult } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { Config, Provider, RepoPRs, PREvent } from "../types";
import { providerCreds } from "../types";

export type PRQueryResult = UseQueryResult<RepoPRs>;

type PRSnapshot = { title: string; url: string };

export function usePRData(
  config: Config,
  onEvent: (e: PREvent) => void,
): PRQueryResult[] {
  const repos = config.repositories;
  const prevRef = useRef<Record<string, Map<number, PRSnapshot>>>({});
  const onEventRef = useRef(onEvent);
  useEffect(() => { onEventRef.current = onEvent; });

  const queries = useQueries({
    queries: repos.map((repo) => {
      const { token, baseUrl } = providerCreds(config, repo);
      return {
        queryKey: ["prs", repo.provider, repo.owner, repo.name],
        queryFn: () =>
          invoke<RepoPRs>("fetch_repo_prs", {
            provider: repo.provider,
            owner: repo.owner,
            repo: repo.name,
            token,
            baseUrl,
          }),
        refetchInterval: config.refreshInterval * 1000,
        staleTime: 0,
        enabled: token.length > 0,
      };
    }),
  });

  const updatedAtKey = queries.map((q) => q.dataUpdatedAt).join(",");

  useEffect(() => {
    queries.forEach((query, i) => {
      if (!query.data || query.isFetching || !repos[i]) return;
      const { prs } = query.data;
      const repo = repos[i];
      // Displayed as-is in toasts and history, so it stays unprefixed.
      const repoName = `${repo.owner}/${repo.name}`;
      // Internal key only: two sources may host the same owner/name.
      const repoKey = `${repo.provider}:${repoName}`;
      const { token, baseUrl } = providerCreds(config, repo);
      const currentMap = new Map(
        prs.map((pr) => [pr.number, { title: pr.title, url: pr.url }]),
      );
      const prev = prevRef.current[repoKey];

      if (prev) {
        for (const pr of prs) {
          if (!prev.has(pr.number)) {
            onEventRef.current({
              type: "new_pr",
              provider: repo.provider,
              repo: repoName,
              prNumber: pr.number,
              prTitle: pr.title,
              url: pr.url,
            });
          }
        }
        for (const [prevNum, { title: prevTitle, url: prevUrl }] of prev) {
          if (!currentMap.has(prevNum)) {
            checkMergedAndNotify(
              repo.provider,
              repo.owner,
              repo.name,
              prevNum,
              prevTitle,
              prevUrl,
              token,
              baseUrl,
              onEventRef,
            );
          }
        }
      }

      prevRef.current[repoKey] = currentMap;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [updatedAtKey]);

  return queries;
}

async function checkMergedAndNotify(
  provider: Provider,
  owner: string,
  repo: string,
  number: number,
  title: string,
  url: string,
  token: string,
  baseUrl: string,
  onEventRef: React.RefObject<(e: PREvent) => void>,
) {
  try {
    const merged = await invoke<boolean>("check_pr_merged", {
      provider,
      owner,
      repo,
      number,
      token,
      baseUrl,
    });
    if (merged) {
      onEventRef.current?.({
        type: "merged",
        provider,
        repo: `${owner}/${repo}`,
        prNumber: number,
        prTitle: title,
        url,
      });
    }
  } catch {
    // silently ignore check failures
  }
}
