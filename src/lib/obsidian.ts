import { invoke } from "@tauri-apps/api/core";

export interface ObsidianConfig {
  base_url: string;
  api_key: string;
  accept_invalid_certs: boolean;
}

export interface ObsidianConnectionSummary {
  base_url: string;
  authenticated: boolean;
  service: string;
  versions: unknown;
}

export interface ObsidianNoteDetail {
  path: string;
  content: string;
  frontmatter: unknown;
  tags: string[];
  stat: {
    ctime: number | null;
    mtime: number | null;
    size: number | null;
  } | null;
}

export const obsidianConfigure = (config: ObsidianConfig) =>
  invoke<ObsidianConnectionSummary>("obsidian_configure", { config });

export const obsidianStatus = () =>
  invoke<ObsidianConnectionSummary | null>("obsidian_status");

export const obsidianListRoot = () =>
  invoke<string[]>("obsidian_list_root");

export const obsidianGetNote = (path: string) =>
  invoke<ObsidianNoteDetail>("obsidian_get_note", { path });

export const obsidianDisconnect = () => invoke<void>("obsidian_disconnect");
