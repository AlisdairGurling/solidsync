import { createSignal, createResource, Show, For, onMount } from "solid-js";
import {
  obsidianConfigure,
  obsidianStatus,
  obsidianListRoot,
  obsidianGetNote,
  obsidianDisconnect,
  type ObsidianNoteDetail,
} from "../lib/obsidian";

const DEFAULT_BASE_HTTPS = "https://127.0.0.1:27124";
const DEFAULT_BASE_HTTP = "http://127.0.0.1:27123";

export function ObsidianPanel() {
  const [status, { refetch: refetchStatus }] = createResource(obsidianStatus);

  const [baseUrl, setBaseUrl] = createSignal(DEFAULT_BASE_HTTPS);
  const [apiKey, setApiKey] = createSignal("");
  const [acceptSelfSigned, setAcceptSelfSigned] = createSignal(true);

  const [busy, setBusy] = createSignal(false);
  const [errorMsg, setErrorMsg] = createSignal("");

  const [files, setFiles] = createSignal<string[] | null>(null);
  const [selectedPath, setSelectedPath] = createSignal<string | null>(null);
  const [note, setNote] = createSignal<ObsidianNoteDetail | null>(null);

  onMount(() => {
    // If we already had a live connection (e.g. dev hot-reload), load the tree.
    refetchStatus();
  });

  async function connect(e: Event) {
    e.preventDefault();
    setBusy(true);
    setErrorMsg("");
    try {
      await obsidianConfigure({
        base_url: baseUrl().trim(),
        api_key: apiKey().trim(),
        accept_invalid_certs: acceptSelfSigned(),
      });
      await refetchStatus();
      await loadRoot();
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function loadRoot() {
    setErrorMsg("");
    try {
      const list = await obsidianListRoot();
      setFiles(list);
    } catch (e) {
      setErrorMsg(String(e));
    }
  }

  async function openNote(path: string) {
    setSelectedPath(path);
    setNote(null);
    setErrorMsg("");
    try {
      const detail = await obsidianGetNote(path);
      setNote(detail);
    } catch (e) {
      setErrorMsg(String(e));
    }
  }

  async function disconnect() {
    await obsidianDisconnect();
    setFiles(null);
    setNote(null);
    setSelectedPath(null);
    await refetchStatus();
  }

  return (
    <section class="card">
      <h2>Connect Obsidian</h2>

      <Show
        when={status()}
        fallback={
          <>
            <p class="hint">
              Install the{" "}
              <a
                href="https://github.com/coddingtonbear/obsidian-local-rest-api"
                target="_blank"
                rel="noreferrer"
              >
                Local REST API
              </a>{" "}
              plugin in Obsidian, copy your API key from its settings, then
              paste it here. The default port is <code>27124</code> (HTTPS,
              self-signed) or <code>27123</code> (plain HTTP, loopback only).
            </p>

            <form onSubmit={connect}>
              <label for="obs-url">Base URL</label>
              <input
                id="obs-url"
                type="text"
                value={baseUrl()}
                onInput={(e) => setBaseUrl(e.currentTarget.value)}
                spellcheck={false}
                autocorrect="off"
                autocapitalize="off"
                disabled={busy()}
              />

              <div class="chips" style="margin-top:4px">
                <button
                  type="button"
                  class="chip"
                  onClick={() => {
                    setBaseUrl(DEFAULT_BASE_HTTPS);
                    setAcceptSelfSigned(true);
                  }}
                  disabled={busy()}
                >
                  HTTPS :27124
                </button>
                <button
                  type="button"
                  class="chip"
                  onClick={() => {
                    setBaseUrl(DEFAULT_BASE_HTTP);
                    setAcceptSelfSigned(false);
                  }}
                  disabled={busy()}
                >
                  HTTP :27123
                </button>
              </div>

              <label for="obs-key">API key</label>
              <input
                id="obs-key"
                type="password"
                value={apiKey()}
                onInput={(e) => setApiKey(e.currentTarget.value)}
                placeholder="Bearer token from plugin settings"
                disabled={busy()}
              />

              <label class="checkbox">
                <input
                  type="checkbox"
                  checked={acceptSelfSigned()}
                  onChange={(e) => setAcceptSelfSigned(e.currentTarget.checked)}
                  disabled={busy()}
                />
                <span>Accept self-signed certificate (loopback only)</span>
              </label>

              <button
                type="submit"
                class="primary"
                disabled={busy() || !apiKey().trim() || !baseUrl().trim()}
              >
                {busy() ? "Connecting…" : "Connect"}
              </button>
            </form>

            <Show when={errorMsg()}>
              <p class="error">{errorMsg()}</p>
            </Show>
          </>
        }
      >
        {(s) => (
          <>
            <dl class="session">
              <dt>Service</dt>
              <dd><code>{s().service}</code></dd>
              <dt>Base URL</dt>
              <dd><code>{s().base_url}</code></dd>
              <dt>Authenticated</dt>
              <dd>{s().authenticated ? "yes" : "no"}</dd>
            </dl>

            <div class="vault-browser">
              <div class="vault-list">
                <div class="vault-toolbar">
                  <strong>Vault root</strong>
                  <button type="button" class="chip" onClick={loadRoot}>
                    Refresh
                  </button>
                </div>
                <Show
                  when={files()}
                  fallback={<p class="hint">Loading…</p>}
                >
                  <ul class="files">
                    <For each={files()!}>
                      {(f) => (
                        <li
                          classList={{ active: selectedPath() === f }}
                          onClick={() => openNote(f)}
                        >
                          {f}
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </div>

              <div class="vault-preview">
                <Show
                  when={note()}
                  fallback={
                    <p class="hint">
                      {selectedPath()
                        ? "Loading note…"
                        : "Pick a file on the left to preview it."}
                    </p>
                  }
                >
                  {(n) => (
                    <>
                      <h3>{n().path}</h3>
                      <Show when={n().tags?.length}>
                        <p class="tags">
                          {n().tags.map((t) => (
                            <span class="tag">#{t}</span>
                          ))}
                        </p>
                      </Show>
                      <pre class="markdown">{n().content}</pre>
                    </>
                  )}
                </Show>
              </div>
            </div>

            <button type="button" class="secondary" onClick={disconnect}>
              Disconnect Obsidian
            </button>

            <Show when={errorMsg()}>
              <p class="error">{errorMsg()}</p>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}
