import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Config } from "../types";

const DEFAULT_CONFIG: Config = {
  githubToken: "",
  githubUrl: "https://github.sinequa.com",
  refreshInterval: 60,
  repositories: [],
};

export function useConfig() {
  const [config, setConfig] = useState<Config>(DEFAULT_CONFIG);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<Config>("get_config")
      .then(setConfig)
      .finally(() => setLoading(false));
  }, []);

  async function saveConfig(newConfig: Config) {
    await invoke("save_config", { config: newConfig });
    setConfig(newConfig);
  }

  return { config, loading, saveConfig };
}
