# SolidSync

> Bridge your PKM tools to your [Solid](https://solidproject.org) Pod.

SolidSync is a cross-platform desktop app (macOS · Windows · Linux) that pulls data out of personal
knowledge management systems — **Notion, Obsidian, Roam Research, Logseq** —
transforms it to [RDF](https://www.w3.org/RDF/) using standard vocabularies
(SIOC, Schema.org, Dublin Core), and syncs it into your decentralised Solid
Pod. You keep your notes. You keep your graph. Any Solid-compliant app can
read it.

**Status:** active development. Phases 1 & 2 (desktop foundation +
authentication) are working. PKM connectors and RDF transformation are next.

---

## Why

PKM tools each lock your data into a silo: Notion in their cloud
database, Roam in their Clojure graph, Obsidian in local Markdown with custom
syntax (or at the very least isn't soild compatable), Logseq in its own SQLite schema. Moving knowledge between them — or
querying it with your own tools — is painful and lossy.

Solid flips the model: your data lives in a **Pod** you control, described as
Linked Data, and applications ask permission to read or write. SolidSync is
the bridge that gets your existing PKM graph *into* that model, so the rest of
the Solid ecosystem can use it.

## How it works

```
┌───────────────┐  extract   ┌─────────────┐  triplify   ┌──────────────┐
│  PKM tools    │ ─────────▶ │  SolidSync  │ ──────────▶ │   Solid Pod  │
│  (Notion,     │            │  (Tauri +   │  (SIOC +    │              │
│   Obsidian,   │            │   Rust +    │  Schema.org │              │
│   Roam,       │            │   SolidJS)  │  + DC)      │              │
│   Logseq)     │            │             │             │              │
└───────────────┘            └─────────────┘             └──────────────┘
```

### Architecture

- **Desktop shell — [Tauri v2](https://v2.tauri.app/)**: Rust backend, OS-native WebView frontend (WKWebView on macOS, WebView2/Edge on Windows, WebKitGTK on Linux), OS-level process isolation. ~14 MB installed on macOS.
- **Frontend — [SolidJS](https://solidjs.com) + TypeScript**: fine-grained
  reactivity, small bundle, chosen to match the Solid ecosystem's performance
  ethos.
- **Authentication — [Solid-OIDC](https://solidproject.org/TR/oidc)**:
  Dynamic Client Registration, PKCE (S256), DPoP-bound access tokens
  ([RFC 9449](https://datatracker.ietf.org/doc/html/rfc9449)). Tokens live in
  the Rust process, never in the WebView.
- **Deep linking**: custom URL scheme `org.solidsync.app://auth/callback`,
  registered per-platform (`Info.plist` on macOS, registry on Windows,
  `.desktop` MIME on Linux). On Windows/Linux a `tauri-plugin-single-instance`
  handler forwards the URL from a second-launched process to the original.

## Current capabilities

- [x] **Phase 1 — Desktop foundation**
  - Tauri v2 + Rust + SolidJS scaffolding (macOS · Windows · Linux)
  - Granular capability permissions (principle of least privilege)
  - Cross-platform custom URL scheme with single-instance forwarding
  - CI builds for all three platforms on every push (GitHub Actions)
- [x] **Phase 2 — Solid-OIDC authentication**
  - OIDC discovery against any Solid-compliant issuer
  - Dynamic Client Registration with `dpop_bound_access_tokens: true`
  - PKCE verifier + S256 challenge
  - ES256 (P-256) DPoP keypair per session
  - DPoP proof generation with `jti / htm / htu / iat / ath / nonce` claims
  - Deep-link callback handling (warm and cold start)
  - WebID extraction from the ID token

## Roadmap

- [ ] **Phase 3 — PKM connectors**
  - [x] Obsidian via the [Local REST API](https://github.com/coddingtonbear/obsidian-local-rest-api) plugin — connection test, vault listing, note retrieval (Markdown + frontmatter + tags)
  - [ ] Logseq DB-version via its Local HTTP API
  - [ ] Notion via the official REST API
  - [ ] Roam via JSON / EDN export (one-way import; Roam has no public write API)
- [ ] **Phase 4 — Semantic transformation**
  - Map PKM structures to SIOC (`sioc:Post`, `sioc:Item`, `sioc:Container`…)
  - Enrich with Schema.org + Dublin Core
  - Serialize to Turtle / JSON-LD
  - PUT/PATCH to the Pod via authenticated DPoP-bound requests
- [ ] **Phase 5 — Cloud sources & automation** *(post-MVP)*
  - Google Drive / Dropbox OAuth
  - Zapier / IFTTT retrieve-poll integration
- [ ] **Phase 6 — Local AI against your graph** *(post-MVP, Apple Silicon only)*
  - Micro-VMs via Apple's Containerization framework
  - GraphRAG / local LLM running offline over your Pod data
- [ ] **Cross-cutting**
  - Keychain persistence of refresh tokens
  - ID token signature verification
  - DPoP server-nonce retry loop
  - WebID profile document parsing (so users can enter a WebID, not only an issuer)

## Platforms

| OS                   | Status                          | Deep-link routing                               |
|----------------------|---------------------------------|-------------------------------------------------|
| macOS 13+            | Primary target, tested          | Native via `Info.plist`                         |
| Windows 10 / 11      | Supported, requires testing     | Registry + `tauri-plugin-single-instance`       |
| Linux (GTK)          | Supported, requires testing     | `.desktop` MIME + `tauri-plugin-single-instance`|

Windows and Linux builds are produced automatically by the
[`build`](.github/workflows/build.yml) GitHub Action — grab the artifacts
from a recent run if you don't want to build from source.

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) stable (via `rustup`)
- [Node.js](https://nodejs.org) 20+
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Windows: [WebView2 runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (preinstalled on Windows 11; most Windows 10) and the MSVC C++ build tools
- Linux: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`

### Build

```sh
git clone https://github.com/AlisdairGurling/solidsync.git
cd solidsync
npm install
npm run tauri build
```

Bundles land in `src-tauri/target/release/bundle/`:

- **macOS** — `macos/SolidSync.app` (and `.dmg` with `--bundles dmg`)
- **Windows** — `nsis/SolidSync_<ver>_x64-setup.exe`, `msi/SolidSync_<ver>_x64_en-US.msi`
- **Linux** — `deb/solidsync_<ver>_amd64.deb`, `appimage/solidsync_<ver>_amd64.AppImage`

To restrict to specific formats: `npm run tauri build -- --bundles app,dmg`.

### Run in development

```sh
npm run tauri dev
```

Hot-reload for the SolidJS frontend; Rust changes trigger a recompile.

### Signing in

1. Launch **SolidSync**.
2. Pick a provider chip (`solidcommunity.net`, `login.inrupt.com`, …) or
   type your own issuer URL.
3. Click **Sign in** — your default browser opens.
4. Authenticate at the provider.
5. The provider redirects to `org.solidsync.app://auth/callback?…`.
   - **macOS** hands the URL back to the running app natively.
   - **Windows / Linux** launch a second instance; the single-instance plugin
     detects it, forwards the URL to the original process, and the
     deep-link handler fires.
6. SolidSync finishes the PKCE + DPoP token exchange and your WebID appears.

**macOS first-run note:** if the redirect says *"cannot find application"*,
quit SolidSync and relaunch it once from Finder — LaunchServices needs one
full run to register the URL scheme. Moving `SolidSync.app` to
`/Applications` makes this persistent.

**Windows SmartScreen note:** unsigned builds trigger SmartScreen on first
launch. Click **More info → Run anyway**. We'll add code signing once the
project gets broader distribution.

## Solid ecosystem notes

SolidSync aims to be a good citizen of the Solid ecosystem:

- Works with **any** Solid-OIDC-compliant Pod provider — no hard-coded list.
- Uses **Dynamic Client Registration** so you never have to pre-register an
  app-wide client ID; each install gets its own.
- Uses **reverse-DNS** URL scheme (`org.solidsync.app://`) per RFC 8252. A
  branded short scheme like `solidsync://` is rejected by Community Solid
  Server's DCR with `invalid_redirect_uri`.
- Scopes requested: `openid profile offline_access webid`.
- All resource requests will carry a **fresh DPoP proof** bound to the
  per-session ES256 key.

## Project layout

```
solidsync/
├── src/                    # SolidJS + TypeScript frontend
│   ├── App.tsx             # Login UI
│   ├── App.css
│   ├── lib/auth.ts         # Thin invoke() wrappers around Rust commands
│   └── index.tsx
├── src-tauri/              # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json     # Bundle + deep-link config
│   ├── capabilities/       # IPC permission scopes
│   └── src/
│       ├── main.rs
│       ├── lib.rs          # App state, plugin init, setup hook
│       ├── commands.rs     # Tauri commands: begin_login / handle_callback / …
│       ├── error.rs
│       └── auth/
│           ├── discovery.rs  # OIDC discovery + Dynamic Client Registration
│           ├── pkce.rs       # PKCE verifier + S256 challenge
│           ├── dpop.rs       # ES256 keypair + DPoP proof JWT
│           ├── session.rs    # TokenSet
│           └── state.rs      # Pending flows + active session
└── package.json
```

## Contributing

Early days — the codebase is small and the architecture is load-bearing, so
the highest-value contributions right now are:

1. **Provider compatibility testing** — try SolidSync against your Pod
   provider and open an issue if auth fails.
2. **WebID profile parser** — read a WebID Profile Document (Turtle / JSON-LD)
   and extract the `solid:oidcIssuer` so users can paste a WebID instead of
   an issuer URL.
3. **Keychain persistence** — store the refresh token in the macOS Keychain
   so sessions survive app restarts.
4. **First PKM connector** (Obsidian Local REST API) — the smallest
   well-defined Phase 3 task.

Before opening a PR, run both:

```sh
cd src-tauri && cargo check
npx tsc --noEmit
```

## License

[MIT](LICENSE) © SolidSync contributors

## Acknowledgements

- The [Solid team](https://solidproject.org/about) and [Inrupt](https://www.inrupt.com/)
  for the protocol, spec work, and reference implementations.
- The [Tauri](https://tauri.app) team for an actually lightweight desktop
  framework.
