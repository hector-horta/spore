use std::sync::Arc;
use reqwest::Client;
use tokio::sync::RwLock;
use crate::auth::{
    build_authorize_url, exchange_code_for_tokens, listen_for_code, refresh_access_token,
    PkceSession, SpotifyTokens, DEFAULT_SPOTIFY_CLIENT_ID,
};
use crate::error::{SporeError, SporeResult};
use crate::lyrics::LyricLine;
use crate::models::{Track, UserPlaylist};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionType {
    Home,
    NewReleases,
    Radio,
}

#[derive(Debug, Clone)]
pub enum AudioStreamSource {
    RawEncoded { bytes: Vec<u8>, extension: &'static str },
    PcmDirect { samples: Vec<i16>, sample_rate: u32, channels: u16 },
}

pub trait StreamingBackend: Send + Sync {
    fn service_name(&self) -> &'static str;
    fn search(&self, query: &str, limit: usize) -> impl std::future::Future<Output = SporeResult<Vec<Track>>> + Send;
    fn fetch_section(&self, section: SectionType) -> impl std::future::Future<Output = SporeResult<Vec<Track>>> + Send;
    fn fetch_user_library(&self) -> impl std::future::Future<Output = SporeResult<Vec<Track>>> + Send;
    fn fetch_user_playlists(&self) -> impl std::future::Future<Output = SporeResult<Vec<UserPlaylist>>> + Send;
    fn fetch_playlist_tracks(&self, playlist_id: &str) -> impl std::future::Future<Output = SporeResult<Vec<Track>>> + Send;
    fn resolve_stream(&self, track: &Track) -> impl std::future::Future<Output = SporeResult<AudioStreamSource>> + Send;
    fn fetch_lyrics(&self, track: &Track) -> impl std::future::Future<Output = SporeResult<Vec<LyricLine>>> + Send;
    fn login(&mut self) -> impl std::future::Future<Output = SporeResult<()>> + Send;
    fn is_logged_in(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct SpotifyBackend {
    pub http: Client,
    pub tokens: Arc<RwLock<Option<SpotifyTokens>>>,
}

impl SpotifyBackend {
    pub fn new() -> SporeResult<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(SporeError::Network)?;
        let loaded = SpotifyTokens::load_from_disk();
        Ok(Self {
            http,
            tokens: Arc::new(RwLock::new(loaded)),
        })
    }

    pub fn client_id(&self) -> String {
        std::env::var("SPORE_SPOTIFY_CLIENT_ID")
            .or_else(|_| std::env::var("SPOTIFY_CLIENT_ID"))
            .unwrap_or_else(|_| DEFAULT_SPOTIFY_CLIENT_ID.to_string())
    }

    pub async fn ensure_valid_token(&self) -> Option<String> {
        let mut guard = self.tokens.write().await;
        let tokens = guard.as_mut()?;
        if tokens.is_expired() {
            if let Some(ref refresh) = tokens.refresh_token.clone() {
                if let Ok(new_tokens) = refresh_access_token(&self.http, &tokens.client_id, refresh).await {
                    let _ = new_tokens.save_to_disk();
                    *tokens = new_tokens;
                }
            }
        }
        Some(tokens.access_token.clone())
    }

    async fn get_authed_json(&self, url: &str) -> SporeResult<serde_json::Value> {
        let Some(token) = self.ensure_valid_token().await else {
            return Err(SporeError::Auth("Inicia sesión presionando 'u'.".to_string()));
        };
        let mut resp = self.http.get(url).bearer_auth(&token).send().await.map_err(SporeError::Network)?;
        if resp.status().as_u16() == 429 {
            let wait = resp.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(3);
            if wait <= 15 {
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                resp = self.http.get(url).bearer_auth(&token).send().await.map_err(SporeError::Network)?;
            }
            if resp.status().as_u16() == 429 {
                return Err(SporeError::Spotify(format!("Cuota pública saturada (429). Reintenta en {wait}s")));
            }
        }
        if !resp.status().is_success() {
            return Err(SporeError::Spotify(format!("Spotify API error: {}", resp.status())));
        }
        resp.json().await.map_err(SporeError::Network)
    }

    fn open_browser(url: &str) {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("rundll32.exe")
                .args(["url.dll,FileProtocolHandler", url])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(url).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = std::process::Command::new("xdg-open").arg(url).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    }
}

impl StreamingBackend for SpotifyBackend {
    fn service_name(&self) -> &'static str {
        "Spotify"
    }

    async fn search(&self, query: &str, limit: usize) -> SporeResult<Vec<Track>> {
        if query.trim().is_empty() { return Ok(Vec::new()); }

        if let Some(token) = self.ensure_valid_token().await {
            let url = format!("https://api.spotify.com/v1/search?q={}&type=track&limit={}", urlencoding(query), limit.min(50));
            if let Ok(resp) = self.http.get(&url).bearer_auth(&token).send().await {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        let mut tracks = Vec::new();
                        if let Some(items) = json.pointer("/tracks/items").and_then(|v| v.as_array()) {
                            for item in items {
                                if let Some(t) = parse_spotify_track(item) { tracks.push(t); }
                            }
                        }
                        if !tracks.is_empty() { return Ok(tracks); }
                    }
                }
            }
        }

        // Fallback open catalog search
        let url = format!("https://itunes.apple.com/search?term={}&entity=song&limit={}", urlencoding(query), limit);
        let resp = self.http.get(&url).send().await.map_err(SporeError::Network)?;
        let json: serde_json::Value = resp.json().await.map_err(SporeError::Network)?;
        let mut tracks = Vec::new();
        if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
            for item in results {
                let id = item.get("trackId").and_then(|v| v.as_i64()).map(|n| n.to_string()).unwrap_or_default();
                let title = item.get("trackName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let artist = item.get("artistName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let album = item.get("collectionName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let duration_ms = item.get("trackTimeMillis").and_then(|v| v.as_u64()).unwrap_or(0);
                let artwork_url = item.get("artworkUrl100").and_then(|v| v.as_str()).map(|s| s.replace("100x100bb", "600x600bb"));
                let preview_url = item.get("previewUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
                if !id.is_empty() && !title.is_empty() {
                    tracks.push(Track { id, title, artist, album, duration_ms, storefront: "us".to_string(), preview_url, artwork_url, hls_manifest_url: None, is_lossless: false });
                }
            }
        }
        Ok(tracks)
    }

    async fn fetch_section(&self, section: SectionType) -> SporeResult<Vec<Track>> {
        match section {
            SectionType::Home => self.search("top hits 2026", 50).await,
            SectionType::NewReleases => self.search("new music friday", 50).await,
            SectionType::Radio => self.search("lo-fi beats chill", 50).await,
        }
    }

    async fn fetch_user_library(&self) -> SporeResult<Vec<Track>> {
        let json = self.get_authed_json("https://api.spotify.com/v1/me/tracks?limit=50").await?;
        let mut tracks = Vec::new();
        if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(tr) = item.get("track").and_then(parse_spotify_track) { tracks.push(tr); }
            }
        }
        Ok(tracks)
    }

    async fn fetch_user_playlists(&self) -> SporeResult<Vec<UserPlaylist>> {
        let json = self.get_authed_json("https://api.spotify.com/v1/me/playlists?limit=50").await?;
        let mut playlists = Vec::new();
        if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
            for item in items {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let description = item.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                let track_count = item.pointer("/tracks/total").and_then(|v| v.as_u64()).map(|n| n as usize);
                if !id.is_empty() { playlists.push(UserPlaylist { id, name, description, track_count }); }
            }
        }
        Ok(playlists)
    }

    async fn fetch_playlist_tracks(&self, playlist_id: &str) -> SporeResult<Vec<Track>> {
        let url = format!("https://api.spotify.com/v1/playlists/{}/tracks?limit=50", playlist_id);
        let json = self.get_authed_json(&url).await?;
        let mut tracks = Vec::new();
        if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(tr) = item.get("track").and_then(parse_spotify_track) { tracks.push(tr); }
            }
        }
        Ok(tracks)
    }

    async fn resolve_stream(&self, track: &Track) -> SporeResult<AudioStreamSource> {
        let stream_url = if let Some(ref p) = track.preview_url {
            Some(p.clone())
        } else {
            let q = format!("{} {}", track.artist, track.title);
            let itunes = format!("https://itunes.apple.com/search?term={}&entity=song&limit=1", urlencoding(&q));
            let r = self.http.get(&itunes).send().await.ok();
            let j: Option<serde_json::Value> = match r { Some(res) => res.json().await.ok(), None => None };
            j.as_ref().and_then(|v| v.pointer("/results/0/previewUrl")).and_then(|s| s.as_str()).map(|s| s.to_string())
        };

        if let Some(url) = stream_url {
            let resp = self.http.get(&url).send().await.map_err(SporeError::Network)?;
            let bytes = resp.bytes().await.map_err(SporeError::Network)?;
            return Ok(AudioStreamSource::RawEncoded { bytes: bytes.to_vec(), extension: "m4a" });
        }
        Err(SporeError::Spotify("Audio stream no disponible para esta canción".to_string()))
    }

    async fn fetch_lyrics(&self, track: &Track) -> SporeResult<Vec<LyricLine>> {
        let lines = crate::lyrics::fetch_lrclib_lyrics(&self.http, &track.title, &track.artist).await;
        Ok(lines)
    }

    async fn login(&mut self) -> SporeResult<()> {
        let client_id = self.client_id();
        let pkce = PkceSession::new();
        let auth_url = build_authorize_url(&client_id, &pkce);
        Self::open_browser(&auth_url);

        let code = listen_for_code(8888, 60).await.map_err(|e| {
            SporeError::Auth(format!("{e}. Asegúrate de autorizar Spore en tu navegador"))
        })?;

        let tokens = exchange_code_for_tokens(&self.http, &client_id, &code, &pkce.verifier).await?;
        tokens.save_to_disk()?;
        *self.tokens.write().await = Some(tokens);
        Ok(())
    }

    fn is_logged_in(&self) -> bool {
        self.tokens.try_read().map(|g| g.is_some()).unwrap_or(false)
    }
}

fn parse_spotify_track(item: &serde_json::Value) -> Option<Track> {
    let id = item.get("id").and_then(|v| v.as_str())?.to_string();
    let title = item.get("name").and_then(|v| v.as_str())?.to_string();
    let artist = item.pointer("/artists/0/name").and_then(|v| v.as_str()).unwrap_or("Artista Desconocido").to_string();
    let album = item.pointer("/album/name").and_then(|v| v.as_str()).unwrap_or("Álbum").to_string();
    let duration_ms = item.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let artwork_url = item.pointer("/album/images/0/url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let preview_url = item.get("preview_url").and_then(|v| v.as_str()).map(|s| s.to_string());
    Some(Track { id, title, artist, album, duration_ms, storefront: "us".to_string(), preview_url, artwork_url, hls_manifest_url: None, is_lossless: false })
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
    fn test_spotify_backend_metadata() {
        let backend = SpotifyBackend::new().unwrap();
        assert_eq!(backend.service_name(), "Spotify");
    }
}
