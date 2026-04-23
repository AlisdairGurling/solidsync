use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::discovery::{OidcConfiguration, RegisteredClient};
use super::dpop::DpopKey;
use super::pkce::PkcePair;
use super::session::TokenSet;

/// State of a login attempt that has kicked off a browser hand-off but has
/// not yet received the authorization-code callback.
pub struct PendingFlow {
    pub issuer: String,
    pub config: OidcConfiguration,
    pub client: RegisteredClient,
    pub pkce: PkcePair,
    pub dpop: DpopKey,
    pub redirect_uri: String,
}

/// A completed, authenticated session.
pub struct ActiveSession {
    pub webid: Option<String>,
    pub issuer: String,
    pub config: OidcConfiguration,
    pub client: RegisteredClient,
    pub dpop: DpopKey,
    pub tokens: TokenSet,
}

#[derive(Default)]
pub struct AuthState {
    /// Pending flows keyed by the OAuth `state` parameter (also serves as CSRF token).
    pub pending: HashMap<String, PendingFlow>,
    pub active: Option<ActiveSession>,
}

pub type SharedAuthState = Arc<RwLock<AuthState>>;
