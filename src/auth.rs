use axum::http::{header, HeaderMap};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

use crate::config::Config;

const COOKIE_NAME: &str = "rsa_session";
const SESSION_TTL_SECS: u64 = 86400 * 7;

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize, Deserialize)]
struct SessionPayload {
    exp: u64,
}

pub fn sign_session(config: &Config) -> Result<String, String> {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs()
        + SESSION_TTL_SECS;
    let payload = SessionPayload { exp };
    let json = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let mut mac = HmacSha256::new_from_slice(&config.session_secret).map_err(|e| e.to_string())?;
    mac.update(&json);
    let sig = mac.finalize().into_bytes();
    let mut token = URL_SAFE_NO_PAD.encode(&json);
    token.push('.');
    token.push_str(&hex::encode(sig));
    Ok(token)
}

pub fn verify_session_token(config: &Config, token: &str) -> bool {
    let Some((b64_json, sig_hex)) = token.split_once('.') else {
        return false;
    };
    let Ok(json) = URL_SAFE_NO_PAD.decode(b64_json) else {
        return false;
    };
    let Ok(sig_expected) = hex::decode(sig_hex) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(&config.session_secret) else {
        return false;
    };
    mac.update(&json);
    let computed = mac.finalize().into_bytes();
    if sig_expected.len() != computed.len() {
        return false;
    }
    if !bool::from(sig_expected.as_slice().ct_eq(computed.as_slice())) {
        return false;
    }
    let Ok(payload): Result<SessionPayload, _> = serde_json::from_slice(&json) else {
        return false;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    payload.exp > now
}

pub fn session_cookie_value(headers: &HeaderMap) -> Option<String> {
    let cookie_hdr = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie_hdr.split(';') {
        let part = part.trim();
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        if name.trim() == COOKIE_NAME {
            return Some(value.trim().to_string());
        }
    }
    None
}

pub fn set_session_cookie(config: &Config) -> Result<String, String> {
    let token = sign_session(config)?;
    // Secure omitted so http://127.0.0.1 works in dev; put HTTPS at the reverse proxy.
    Ok(format!(
        "{COOKIE_NAME}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={SESSION_TTL_SECS}"
    ))
}

pub fn clear_session_cookie_header_value() -> &'static str {
    concat!(
        "rsa_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
    )
}
