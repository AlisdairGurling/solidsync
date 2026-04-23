use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use rand::rngs::OsRng;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{Result, SolidSyncError};

/// ES256 (P-256) signing key used for DPoP proofs, plus the public JWK
/// representation needed for the `jwk` JOSE header on every proof.
///
/// One key is generated per session and reused for every resource request,
/// as required by RFC 9449.
pub struct DpopKey {
    signing_key: SigningKey,
    jwk: serde_json::Value,
}

impl DpopKey {
    pub fn new() -> Self {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let point = verifying_key.to_encoded_point(false);
        let x = point.x().expect("P-256 point has x");
        let y = point.y().expect("uncompressed P-256 point has y");
        let jwk = json!({
            "kty": "EC",
            "crv": "P-256",
            "x": URL_SAFE_NO_PAD.encode(x),
            "y": URL_SAFE_NO_PAD.encode(y),
        });
        Self { signing_key, jwk }
    }

    pub fn jwk(&self) -> &serde_json::Value {
        &self.jwk
    }

    /// Build a DPoP proof JWT.
    ///
    /// * `htm` — the HTTP method, upper-case ("GET", "POST", ...).
    /// * `htu` — the full request URL (minus query/fragment per RFC 9449).
    /// * `access_token` — when bound to an existing access token, include the
    ///   `ath` claim (SHA-256 hash of the token). Pass `None` for the initial
    ///   token request.
    /// * `nonce` — server-supplied DPoP nonce, if the provider requested one.
    pub fn proof(
        &self,
        htm: &str,
        htu: &str,
        access_token: Option<&str>,
        nonce: Option<&str>,
    ) -> Result<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| SolidSyncError::Crypto(e.to_string()))?
            .as_secs() as i64;

        let header = json!({
            "typ": "dpop+jwt",
            "alg": "ES256",
            "jwk": self.jwk,
        });

        let mut claims = serde_json::Map::new();
        claims.insert("jti".into(), json!(Uuid::new_v4().to_string()));
        claims.insert("htm".into(), json!(htm));
        claims.insert("htu".into(), json!(htu));
        claims.insert("iat".into(), json!(now));
        if let Some(tok) = access_token {
            let ath = URL_SAFE_NO_PAD.encode(Sha256::digest(tok.as_bytes()));
            claims.insert("ath".into(), json!(ath));
        }
        if let Some(n) = nonce {
            claims.insert("nonce".into(), json!(n));
        }

        let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
        let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&serde_json::Value::Object(claims))?);
        let signing_input = format!("{}.{}", header_b64, payload_b64);

        let signature: Signature = self.signing_key.sign(signing_input.as_bytes());
        let sig_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

        Ok(format!("{}.{}", signing_input, sig_b64))
    }
}
