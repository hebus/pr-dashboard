import { useEffect, useRef } from "react";
import { useQueries } from "@tanstack/react-query";
import type { UseQueryResult } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { Config, RepoPRs, PREvent } from "../types";

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
    queries: repos.map((repo) => ({
      queryKey: ["prs", repo.owner, repo.name],
      queryFn: () =>
        invoke<RepoPRs>("fetch_repo_prs", {
          owner: repo.owner,
          repo: repo.name,
          token: config.githubToken,
          githubUrl: config.githubUrl,
        }),
      refetchInterval: config.refreshInterval * 1000,
      staleTime: 0,
      enabled: config.githubToken.length > 0,
    })),
  });

  const updatedAtKey = queries.map((q) => q.dataUpdatedAt).join(",");

  useEffect(() => {
    queries.forEach((query, i) => {
      if (!query.data || query.isFetching || !repos[i]) return;
      const { prs } = query.data;
      const repo = repos[i];
      const repoKey = `${repo.owner}/${repo.name}`;
      const currentMap = new Map(
        prs.map((pr) => [pr.number, { title: pr.title, url: pr.url }]),
      );
      const prev = prevRef.current[repoKey];

      if (prev) {
        for (const pr of prs) {
          if (!prev.has(pr.number)) {
            onEventRef.current({
              type: "new_pr",
              repo: repoKey,
              prNumber: pr.number,
              prTitle: pr.title,
              url: pr.url,
            });
          }
        }
        for (const [prevNum, { title: prevTitle, url: prevUrl }] of prev) {
          if (!currentMap.has(prevNum)) {
            checkMergedAndNotify(
              repo.owner,
              repo.name,
              prevNum,
              prevTitle,
              prevUrl,
              config.githubToken,
              config.githubUrl,
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
  owner: string,
  repo: string,
  number: number,
  title: string,
  url: string,
  token: string,
  githubUrl: string,
  onEventRef: React.RefObject<(e: PREvent) => void>,
) {
  try {
    const merged = await invoke<boolean>("check_pr_merged", {
      owner,
      repo,
      number,
      token,
      githubUrl,
    });
    if (merged) {
      onEventRef.current?.({
        type: "merged",
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
