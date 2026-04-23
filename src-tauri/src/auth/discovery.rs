use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Result, SolidSyncError};

/// Subset of the OIDC discovery document we actually use.
/// Unknown fields are ignored so any Solid-OIDC provider parses.
#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfiguration {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: Option<String>,
    pub jwks_uri: Option<String>,
    #[serde(default)]
    pub scopes_supported: Vec<String>,
    #[serde(default)]
    pub response_types_supported: Vec<String>,
    #[serde(default)]
    pub grant_types_supported: Vec<String>,
    #[serde(default)]
    pub code_challenge_methods_supported: Vec<String>,
    #[serde(default)]
    pub dpop_signing_alg_values_supported: Vec<String>,
}

pub async fn discover(client: &Client, issuer: &str) -> Result<OidcConfiguration> {
    let base = Url::parse(issuer)?;
    // Spec: append /.well-known/openid-configuration — preserve trailing slash handling.
    let mut path = base.path().trim_end_matches('/').to_string();
    path.push_str("/.well-known/openid-configuration");
    let mut url = base.clone();
    url.set_path(&path);

    let resp = client.get(url.clone()).send().await?;
    if !resp.status().is_success() {
        return Err(SolidSyncError::Discovery(format!(
            "{} returned {}",
            url,
            resp.status()
        )));
    }
    Ok(resp.json::<OidcConfiguration>().await?)
}

#[derive(Debug, Serialize)]
struct RegistrationRequest<'a> {
    client_name: &'a str,
    redirect_uris: Vec<&'a str>,
    grant_types: Vec<&'a str>,
    response_types: Vec<&'a str>,
    application_type: &'a str,
    token_endpoint_auth_method: &'a str,
    scope: &'a str,
    dpop_bound_access_tokens: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegisteredClient {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_id_issued_at: Option<u64>,
    pub client_secret_expires_at: Option<u64>,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    pub registration_access_token: Option<String>,
    pub registration_client_uri: Option<String>,
}

pub async fn register_client(
    client: &Client,
    config: &OidcConfiguration,
    redirect_uri: &str,
) -> Result<RegisteredClient> {
    let endpoint = config.registration_endpoint.as_deref().ok_or_else(|| {
        SolidSyncError::Registration(
            "provider does not advertise a registration_endpoint — Dynamic Client Registration is required".into(),
        )
    })?;

    let body = RegistrationRequest {
        client_name: "SolidSync",
        redirect_uris: vec![redirect_uri],
        grant_types: vec!["authorization_code", "refresh_token"],
        response_types: vec!["code"],
        application_type: "native",
        token_endpoint_auth_method: "none",
        scope: "openid profile offline_access webid",
        dpop_bound_access_tokens: true,
    };

    let resp = client.post(endpoint).json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(SolidSyncError::Registration(format!("{}: {}", status, text)));
    }
    Ok(resp.json::<RegisteredClient>().await?)
}
