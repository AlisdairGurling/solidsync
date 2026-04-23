import { createSignal, createResource, Show, onMount, onCleanup } from "solid-js";
import { onOpenUrl, getCurrent } from "@tauri-apps/plugin-deep-link";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  beginLogin,
  currentSession,
  handleCallback,
  logout,
  type SessionSummary,
} from "./lib/auth";
import "./App.css";

const KNOWN_PROVIDERS: { label: string; issuer: string }[] = [
  { label: "solidcommunity.net", issuer: "https://solidcommunity.net" },
  { label: "inrupt.net", issuer: "https://inrupt.net" },
  { label: "login.inrupt.com", issuer: "https://login.inrupt.com" },
  { label: "solidweb.org", issuer: "https://solidweb.org" },
];

function App() {
  const [issuer, setIssuer] = createSignal("https://solidcommunity.net");
  const [status, setStatus] = createSignal<string>("");
  const [errorMsg, setErrorMsg] = createSignal<string>("");
  const [busy, setBusy] = createSignal(false);

  const [session, { refetch: refetchSession }] =
    createResource<SessionSummary | null>(currentSession);

  const processCallback = async (url: string) => {
    setStatus("Exchanging authorization code…");
    setErrorMsg("");
    try {
      await handleCallback(url);
      setStatus("Signed in.");
      await refetchSession();
    } catch (e) {
      setErrorMsg(String(e));
      setStatus("");
    }
  };

  onMount(async () => {
    // The deep-link plugin delivers the solidsync://auth/callback URL back
    // to us after the user authenticates in the browser.
    let unlisten: UnlistenFn | undefined;
    try {
      unlisten = await onOpenUrl((urls) => {
        const url = urls?.[0];
        if (url) processCallback(url);
      });
    } catch (e) {
      console.warn("deep-link onOpenUrl failed", e);
    }
    onCleanup(() => unlisten?.());

    // If the app was launched *by* the deep link (cold-start), pick it up.
    try {
      const pending = await getCurrent();
      const url = pending?.[0];
      if (url) processCallback(url);
    } catch (e) {
      console.warn("deep-link getCurrent failed", e);
    }
  });

  async function signIn() {
    setErrorMsg("");
    setBusy(true);
    setStatus("Contacting identity provider…");
    try {
      await beginLogin(issuer());
      setStatus("Opened your browser. Complete sign-in there, then return here.");
    } catch (e) {
      setErrorMsg(String(e));
      setStatus("");
    } finally {
      setBusy(false);
    }
  }

  async function signOut() {
    await logout();
    await refetchSession();
    setStatus("Signed out.");
  }

  return (
    <main class="container">
      <header>
        <h1>SolidSync</h1>
        <p class="tagline">Bring your PKM into your Solid Pod.</p>
      </header>

      <Show
        when={session()}
        fallback={
          <section class="card">
            <h2>Sign in to your Solid Pod</h2>
            <p class="hint">
              Enter your identity provider. We'll open your browser, you'll log
              in there, and return back here automatically.
            </p>

            <div class="chips">
              {KNOWN_PROVIDERS.map((p) => (
                <button
                  type="button"
                  class="chip"
                  onClick={() => setIssuer(p.issuer)}
                  disabled={busy()}
                >
                  {p.label}
                </button>
              ))}
            </div>

            <form
              onSubmit={(e) => {
                e.preventDefault();
                signIn();
              }}
            >
              <label for="issuer">Identity provider</label>
              <input
                id="issuer"
                type="text"
                value={issuer()}
                onInput={(e) => setIssuer(e.currentTarget.value)}
                placeholder="https://solidcommunity.net"
                spellcheck={false}
                autocorrect="off"
                autocapitalize="off"
                disabled={busy()}
              />
              <button type="submit" class="primary" disabled={busy() || !issuer().trim()}>
                {busy() ? "Working…" : "Sign in"}
              </button>
            </form>

            <Show when={status()}>
              <p class="status">{status()}</p>
            </Show>
            <Show when={errorMsg()}>
              <p class="error">{errorMsg()}</p>
            </Show>
          </section>
        }
      >
        {(sess) => (
          <section class="card">
            <h2>Signed in</h2>
            <dl class="session">
              <dt>WebID</dt>
              <dd>
                <Show when={sess().webid} fallback={<em>not provided by issuer</em>}>
                  <code>{sess().webid}</code>
                </Show>
              </dd>
              <dt>Issuer</dt>
              <dd><code>{sess().issuer}</code></dd>
              <dt>Client ID</dt>
              <dd><code class="small">{sess().client_id}</code></dd>
              <dt>Scope</dt>
              <dd><code class="small">{sess().scope ?? "(n/a)"}</code></dd>
              <dt>Expires</dt>
              <dd>
                <Show
                  when={sess().expires_at}
                  fallback={<em>unknown</em>}
                >
                  {new Date(sess().expires_at! * 1000).toLocaleString()}
                </Show>
              </dd>
            </dl>
            <button type="button" class="secondary" onClick={signOut}>
              Sign out
            </button>
            <Show when={status()}>
              <p class="status">{status()}</p>
            </Show>
          </section>
        )}
      </Show>

      <footer>
        <p>
          SolidSync speaks the Solid-OIDC protocol with PKCE + DPoP. Works with
          any compliant Pod provider.
        </p>
      </footer>
    </main>
  );
}

export default App;
