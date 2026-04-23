use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub token_type: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_in: Option<u64>,
    pub scope: Option<String>,
    /// Unix seconds when `access_token` becomes invalid. Computed client-side
    /// from `obtained_at + expires_in` so the frontend can show countdown /
    /// trigger a refresh without re-asking the server.
    pub expires_at: Option<i64>,
}
