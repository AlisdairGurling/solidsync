use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::auth::discovery::{discover, register_client};
use crate::auth::dpop::DpopKey;
use crate::auth::pkce::PkcePair;
use crate::auth::session::TokenSet;
use crate::auth::state::{ActiveSession, PendingFlow};
use crate::connectors::obsidian::{
    NoteDetail, ObsidianClient, ObsidianConfig, ObsidianStatus,
};
use crate::error::{Result, SolidSyncError};
use crate::{AppState, ObsidianClientState};

pub const REDIRECT_URI: &str = "org.solidsync.app://auth/callback";
pub const SCOPES: &str = "openid profile offline_access webid";

#[derive(Debug, Serialize, Clone)]
pub struct SessionSummary {
    pub webid: Option<String>,
    pub issuer: String,
    pub client_id: String,
    pub expires_at: Option<i64>,
    pub scope: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BeginLoginResponse {
    pub auth_url: String,
    pub state: String,
}

/// Kick off the Solid-OIDC login flow:
/// 1. discover the issuer's OIDC configuration
/// 2. dynamically register ourselves as a native client
/// 3. generate PKCE + DPoP material
/// 4. open the user's default browser at the authorization endpoint
#[tauri::command]
pub async fn begin_login(
    issuer: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BeginLoginResponse> {
    let issuer = normalize_issuer(&issuer)?;
    tracing::info!(issuer = %issuer, "begin_login");

    let config = discover(&state.http, &issuer).await?;
    let client = register_client(&state.http, &config, REDIRECT_URI).await?;
    let pkce = PkcePair::new();
    let dpop = DpopKey::new();

    let csrf_state = uuid::Uuid::new_v4().to_string();

    let mut auth_url = Url::parse(&config.authorization_endpoint)?;
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &client.client_id)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("scope", SCOPES)
        .append_pair("state", &csrf_state)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256");

    {
        let mut auth = state.auth.write().await;
        auth.pending.insert(
            csrf_state.clone(),
            PendingFlow {
                issuer: issuer.clone(),
                config,
                client,
                pkce,
                dpop,
                redirect_uri: REDIRECT_URI.to_string(),
            },
        );
    }

    app.opener()
        .open_url(auth_url.as_str(), None::<&str>)
        .map_err(|e| SolidSyncError::Other(format!("failed to open browser: {e}")))?;

    Ok(BeginLoginResponse {
        auth_url: auth_url.into(),
        state: csrf_state,
    })
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
}

/// Handle the deep-link callback (solidsync://auth/callback?code=...&state=...).
/// Exchanges the authorization code for a DPoP-bound access token.
#[tauri::command]
pub async fn handle_callback(
    url: String,
    state: State<'_, AppState>,
) -> Result<SessionSummary> {
    tracing::info!(url = %url, "handle_callback");
    let parsed = Url::parse(&url)?;

    let mut code: Option<String> = None;
    let mut csrf_state: Option<String> = None;
    let mut oauth_error: Option<String> = None;
    let mut oauth_error_desc: Option<String> = None;
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => csrf_state = Some(v.into_owned()),
            "error" => oauth_error = Some(v.into_owned()),
            "error_description" => oauth_error_desc = Some(v.into_owned()),
            _ => {}
        }
    }
    if let Some(err) = oauth_error {
        return Err(SolidSyncError::Token(match oauth_error_desc {
            Some(d) => format!("{err}: {d}"),
            None => err,
        }));
    }
    let code = code.ok_or_else(|| SolidSyncError::FlowState("callback missing `code`".into()))?;
    let csrf_state = csrf_state
        .ok_or_else(|| SolidSyncError::FlowState("callback missing `state`".into()))?;

    let pending = {
        let mut auth = state.auth.write().await;
        auth.pending.remove(&csrf_state).ok_or_else(|| {
            SolidSyncError::FlowState(
                "unknown `state` — no pending login matches this callback".into(),
            )
        })?
    };

    let dpop_proof = pending
        .dpop
        .proof("POST", &pending.config.token_endpoint, None, None)?;

    let form = [
        ("grant_type", "authorization_code"),
        ("code", code.as_str()),
        ("code_verifier", pending.pkce.verifier.as_str()),
        ("client_id", pending.client.client_id.as_str()),
        ("redirect_uri", pending.redirect_uri.as_str()),
    ];

    let resp = state
        .http
        .post(&pending.config.token_endpoint)
        .header("DPoP", dpop_proof)
        .form(&form)
        .send()
        .await?;

    let tr: TokenResponse = if resp.status().is_success() {
        resp.json().await?
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(SolidSyncError::Token(format!("{status}: {text}")));
    };

    let webid = tr.id_token.as_deref().and_then(extract_webid_from_id_token);

    let obtained_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let tokens = TokenSet {
        access_token: tr.access_token,
        token_type: tr.token_type,
        refresh_token: tr.refresh_token,
        id_token: tr.id_token,
        expires_in: tr.expires_in,
        scope: tr.scope.clone(),
        expires_at: tr.expires_in.map(|s| obtained_at + s as i64),
    };

    let summary = SessionSummary {
        webid: webid.clone(),
        issuer: pending.issuer.clone(),
        client_id: pending.client.client_id.clone(),
        expires_at: tokens.expires_at,
        scope: tokens.scope.clone(),
    };

    {
        let mut auth = state.auth.write().await;
        auth.active = Some(ActiveSession {
            webid,
            issuer: pending.issuer,
            config: pending.config,
            client: pending.client,
            dpop: pending.dpop,
            tokens,
        });
    }

    tracing::info!(webid = ?summary.webid, "login complete");
    Ok(summary)
}

#[tauri::command]
pub async fn current_session(state: State<'_, AppState>) -> Result<Option<SessionSummary>> {
    let auth = state.auth.read().await;
    Ok(auth.active.as_ref().map(|s| SessionSummary {
        webid: s.webid.clone(),
        issuer: s.issuer.clone(),
        client_id: s.client.client_id.clone(),
        expires_at: s.tokens.expires_at,
        scope: s.tokens.scope.clone(),
    }))
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<()> {
    let mut auth = state.auth.write().await;
    auth.active = None;
    auth.pending.clear();
    Ok(())
}

// ----- Obsidian connector -------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ObsidianConnectionSummary {
    pub base_url: String,
    pub authenticated: bool,
    pub service: String,
    pub versions: serde_json::Value,
}

/// Configure (or reconfigure) the Obsidian connection, then immediately verify
/// it by calling the plugin's status endpoint.
#[tauri::command]
pub async fn obsidian_configure(
    config: ObsidianConfig,
    state: State<'_, AppState>,
) -> Result<ObsidianConnectionSummary> {
    tracing::info!(base = %config.base_url, "obsidian_configure");
    let client = ObsidianClient::new(config.clone())?;
    let status: ObsidianStatus = client.status().await?;

    if !status.authenticated {
        return Err(SolidSyncError::Other(
            "Obsidian plugin reached but API key is not accepted (authenticated=false)".into(),
        ));
    }

    let summary = ObsidianConnectionSummary {
        base_url: config.base_url.clone(),
        authenticated: status.authenticated,
        service: status.service.clone(),
        versions: status.versions.clone(),
    };

    *state.obsidian.write().await = Some(ObsidianClientState { config, client });
    Ok(summary)
}

#[tauri::command]
pub async fn obsidian_status(
    state: State<'_, AppState>,
) -> Result<Option<ObsidianConnectionSummary>> {
    let guard = state.obsidian.read().await;
    let Some(ref oc) = *guard else { return Ok(None); };
    let status = oc.client.status().await?;
    Ok(Some(ObsidianConnectionSummary {
        base_url: oc.config.base_url.clone(),
        authenticated: status.authenticated,
        service: status.service,
        versions: status.versions,
    }))
}

#[tauri::command]
pub async fn obsidian_list_root(state: State<'_, AppState>) -> Result<Vec<String>> {
    let guard = state.obsidian.read().await;
    let oc = guard.as_ref().ok_or_else(|| {
        SolidSyncError::Other("Obsidian is not configured".into())
    })?;
    oc.client.list_root().await
}

#[tauri::command]
pub async fn obsidian_get_note(
    path: String,
    state: State<'_, AppState>,
) -> Result<NoteDetail> {
    let guard = state.obsidian.read().await;
    let oc = guard.as_ref().ok_or_else(|| {
        SolidSyncError::Other("Obsidian is not configured".into())
    })?;
    oc.client.get_note(&path).await
}

#[tauri::command]
pub async fn obsidian_disconnect(state: State<'_, AppState>) -> Result<()> {
    *state.obsidian.write().await = None;
    Ok(())
}

// ----- helpers ------------------------------------------------------------

fn normalize_issuer(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SolidSyncError::FlowState("issuer must not be empty".into()));
    }
    let candidate = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let url = Url::parse(&candidate)?;
    // Issuer normalized: no fragment, no trailing slash.
    let mut s = url.as_str().trim_end_matches('/').to_string();
    if let Some(pos) = s.find('#') {
        s.truncate(pos);
    }
    Ok(s)
}

/// Extract the user's WebID from an ID token without verifying its signature.
///
/// This is intentionally a read-only parse: we accept whatever the provider
/// issued. The ID token signature will be verified when we start doing
/// resource requests (follow-up).
fn extract_webid_from_id_token(id_token: &str) -> Option<String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    // Solid-OIDC places the WebID in the `webid` claim; some providers also
    // put it in `sub`.
    json.get("webid")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            json.get("sub")
                .and_then(|v| v.as_str())
                .filter(|s| s.starts_with("http"))
                .map(str::to_string)
        })
}
