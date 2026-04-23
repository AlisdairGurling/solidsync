use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolidSyncError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("OIDC discovery failed: {0}")]
    Discovery(String),
    #[error("Client registration failed: {0}")]
    Registration(String),
    #[error("Token exchange failed: {0}")]
    Token(String),
    #[error("Auth flow state error: {0}")]
    FlowState(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("{0}")]
    Other(String),
}

impl serde::Serialize for SolidSyncError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SolidSyncError>;
