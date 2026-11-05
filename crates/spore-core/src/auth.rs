use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::error::{SporeError, SporeResult};

pub const DEFAULT_SPOTIFY_CLIENT_ID: &str = "01127027ae4d4f8287e0766ef5889ec1";
pub const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
pub const SCOPES: &str = "user-read-private user-read-email playlist-read-private playlist-read-collaborative user-library-read user-read-playback-state user-modify-playback-state";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpotifyTokens {
    pub client_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at_unix: u64,
}

impl SpotifyTokens {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        now + 60 >= self.expires_at_unix
    }

    pub fn auth_file_path() -> PathBuf {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("spore").join("spore_auth.json")
    }

    pub fn load_from_disk() -> Option<Self> {
        let path = Self::auth_file_path();
        let bytes = std::fs::read(&path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn save_to_disk(&self) -> SporeResult<()> {
        let path = Self::auth_file_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = serde_json::to_string_pretty(self).map_err(SporeError::Serialization)?;
        std::fs::write(&path, data).map_err(SporeError::Io)?;
        Ok(())
    }
}

pub struct PkceSession {
    pub verifier: String,
    pub challenge: String,
}

impl PkceSession {
    pub fn new() -> Self {
        let seed = format!("{}:{:?}:spore_pkce", std::process::id(), SystemTime::now());
        let hash = Sha256::digest(seed.as_bytes());
        let verifier = URL_SAFE_NO_PAD.encode(hash);
        let challenge_hash = Sha256::digest(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(challenge_hash);
        Self { verifier, challenge }
    }
}

impl Default for PkceSession {
    fn default() -> Self {
        Self::new()
    }
}

pub fn build_authorize_url(client_id: &str, pkce: &PkceSession) -> String {
    format!(
        "https://accounts.spotify.com/authorize?client_id={}&response_type=code&redirect_uri={}&code_challenge_method=S256&code_challenge={}&scope={}",
        urlencoding(client_id),
        urlencoding(REDIRECT_URI),
        urlencoding(&pkce.challenge),
        urlencoding(SCOPES)
    )
}

pub fn listen_for_code(port: u16, timeout_secs: u64) -> SporeResult<String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(SporeError::Io)?;
    listener.set_nonblocking(false).map_err(SporeError::Io)?;

    let (mut stream, _) = listener.accept().map_err(SporeError::Io)?;
    stream.set_read_timeout(Some(Duration::from_secs(timeout_secs))).map_err(SporeError::Io)?;

    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf).map_err(SporeError::Io)?;
    let req = String::from_utf8_lossy(&buf[..n]);

    let html = "<!DOCTYPE html><html><body style=\"background:#121212;color:#1DB954;font-family:sans-serif;text-align:center;padding:50px;\"><h2>Spore: Autenticacion Exitosa</h2><p style=\"color:#fff;\">Puedes regresar a la terminal.</p></body></html>";
    let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}", html.len(), html);
    let _ = stream.write_all(resp.as_bytes());

    let code = req.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|path| path.split('?').nth(1))
        .and_then(|query| query.split('&').find(|pair| pair.starts_with("code=")))
        .and_then(|pair| pair.strip_prefix("code="))
        .ok_or_else(|| SporeError::Auth("No se encontro el codigo de autorizacion en el callback".to_string()))?;

    Ok(code.to_string())
}

#[derive(Deserialize)]
struct TokenApiResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

pub async fn exchange_code_for_tokens(http: &Client, client_id: &str, code: &str, verifier: &str) -> SporeResult<SpotifyTokens> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("client_id", client_id),
        ("code_verifier", verifier),
    ];
    let res = http.post("https://accounts.spotify.com/api/token")
        .form(&params)
        .send().await.map_err(SporeError::Network)?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(SporeError::Auth(format!("Error canjeando codigo: {body}")));
    }

    let parsed: TokenApiResponse = res.json().await.map_err(SporeError::Network)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    Ok(SpotifyTokens {
        client_id: client_id.to_string(),
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_at_unix: now + parsed.expires_in,
    })
}

pub async fn refresh_access_token(http: &Client, client_id: &str, refresh_token: &str) -> SporeResult<SpotifyTokens> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
    ];
    let res = http.post("https://accounts.spotify.com/api/token")
        .form(&params)
        .send().await.map_err(SporeError::Network)?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(SporeError::Auth(format!("Error refrescando token: {body}")));
    }

    let parsed: TokenApiResponse = res.json().await.map_err(SporeError::Network)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    Ok(SpotifyTokens {
        client_id: client_id.to_string(),
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token.or_else(|| Some(refresh_token.to_string())),
        expires_at_unix: now + parsed.expires_in,
    })
}

fn urlencoding(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = PkceSession::new();
        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_ne!(pkce.verifier, pkce.challenge);
    }

    #[test]
    fn test_build_authorize_url() {
        let pkce = PkceSession::new();
        let url = build_authorize_url("test_client_id", &pkce);
        assert!(url.contains("accounts.spotify.com/authorize"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("test_client_id"));
        assert!(url.contains("127.0.0.1%3A8888"));
    }

    #[test]
    fn test_tokens_expiry() {
        let active = SpotifyTokens {
            client_id: "id".to_string(),
            access_token: "tok".to_string(),
            refresh_token: None,
            expires_at_unix: u64::MAX,
        };
        assert!(!active.is_expired());

        let expired = SpotifyTokens {
            client_id: "id".to_string(),
            access_token: "tok".to_string(),
            refresh_token: None,
            expires_at_unix: 0,
        };
        assert!(expired.is_expired());
    }
}
