import { invoke } from "@tauri-apps/api/core";

export interface SessionSummary {
  webid: string | null;
  issuer: string;
  client_id: string;
  expires_at: number | null;
  scope: string | null;
}

export interface BeginLoginResponse {
  auth_url: string;
  state: string;
}

export const beginLogin = (issuer: string) =>
  invoke<BeginLoginResponse>("begin_login", { issuer });

export const handleCallback = (url: string) =>
  invoke<SessionSummary>("handle_callback", { url });

export const currentSession = () =>
  invoke<SessionSummary | null>("current_session");

export const logout = () => invoke<void>("logout");
