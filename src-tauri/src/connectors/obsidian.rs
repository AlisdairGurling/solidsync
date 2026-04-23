//! Obsidian connector.
//!
//! Talks to the user's local Obsidian vault via the community-developed
//! [Local REST API plugin]. That plugin runs an authenticated HTTP(S) server
//! inside Obsidian itself, so we can list and fetch notes as structured JSON
//! — no proprietary-Markdown parsing required.
//!
//! [Local REST API plugin]: https://github.com/coddingtonbear/obsidian-local-rest-api
//!
//! The plugin's default HTTPS port is **27124** with a self-signed certificate.
//! Users can also enable plain HTTP on **27123** for loopback-only connections.
//! We support both; the connector builds its own `reqwest::Client` so that
//! `danger_accept_invalid_certs` is scoped to this connector and never leaks
//! into our general HTTP client.

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Result, SolidSyncError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianConfig {
    /// e.g. `https://127.0.0.1:27124` (HTTPS, self-signed) or `http://127.0.0.1:27123`.
    pub base_url: String,
    pub api_key: String,
    /// Accept the plugin's self-signed TLS certificate. Only set true for a
    /// loopback address — never for a remote URL.
    #[serde(default)]
    pub accept_invalid_certs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianStatus {
    pub authenticated: bool,
    pub service: String,
    pub versions: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultListing {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteDetail {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub frontmatter: serde_json::Value,
    #[serde(default)]
    pub tags: Vec<String>,
    pub stat: Option<NoteStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteStat {
    pub ctime: Option<i64>,
    pub mtime: Option<i64>,
    pub size: Option<u64>,
}

pub struct ObsidianClient {
    http: Client,
    base: Url,
    api_key: String,
}

impl ObsidianClient {
    pub fn new(config: ObsidianConfig) -> Result<Self> {
        let base = Url::parse(&config.base_url)?;
        // Guard rail: only accept self-signed certs on a loopback host.
        if config.accept_invalid_certs {
            let host = base.host_str().unwrap_or("");
            if host != "127.0.0.1" && host != "localhost" && host != "::1" {
                return Err(SolidSyncError::Other(
                    "accept_invalid_certs is only allowed for loopback addresses".into(),
                ));
            }
        }
        let http = Client::builder()
            .user_agent(concat!("SolidSync/", env!("CARGO_PKG_VERSION"), " (obsidian-connector)"))
            .danger_accept_invalid_certs(config.accept_invalid_certs)
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        Ok(Self {
            http,
            base,
            api_key: config.api_key,
        })
    }

    fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(h) = HeaderValue::from_str(&format!("Bearer {}", self.api_key)) {
            headers.insert(AUTHORIZATION, h);
        }
        headers
    }

    fn url(&self, path: &str) -> Result<Url> {
        // Preserve the base's existing path if any, then append.
        let trimmed = path.trim_start_matches('/');
        self.base
            .join(trimmed)
            .map_err(SolidSyncError::from)
    }

    /// `GET /` — confirms the plugin is up and the API key is accepted.
    pub async fn status(&self) -> Result<ObsidianStatus> {
        let url = self.url("/")?;
        let resp = self
            .http
            .get(url.clone())
            .headers(self.auth_headers())
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(SolidSyncError::Other(format!(
                "Obsidian status check failed ({}): {}",
                status, body
            )));
        }
        serde_json::from_str::<ObsidianStatus>(&body).map_err(SolidSyncError::from)
    }

    /// `GET /vault/` — lists files at the vault root.
    ///
    /// The Local REST API only returns the requested directory's immediate
    /// children; recursion is the caller's job.
    pub async fn list_root(&self) -> Result<Vec<String>> {
        let url = self.url("/vault/")?;
        let resp = self
            .http
            .get(url)
            .headers(self.auth_headers())
            .send()
            .await?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(SolidSyncError::Other(format!(
                "Obsidian list root failed ({}): {}",
                s, t
            )));
        }
        let listing: VaultListing = resp.json().await?;
        Ok(listing.files)
    }

    /// `GET /vault/{path}` with the plugin's structured-note Accept header.
    pub async fn get_note(&self, path: &str) -> Result<NoteDetail> {
        let encoded = encode_path(path);
        let url = self.url(&format!("/vault/{}", encoded))?;
        let resp = self
            .http
            .get(url)
            .headers(self.auth_headers())
            .header(ACCEPT, "application/vnd.olrapi.note+json")
            .send()
            .await?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(SolidSyncError::Other(format!(
                "Obsidian get_note failed ({}): {}",
                s, t
            )));
        }
        Ok(resp.json::<NoteDetail>().await?)
    }
}

/// Percent-encode each path segment while preserving `/` separators.
fn encode_path(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            percent_encoding::utf8_percent_encode(seg, percent_encoding::NON_ALPHANUMERIC)
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("/")
}
