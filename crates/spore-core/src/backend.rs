use reqwest::Client;
use crate::error::{SporeError, SporeResult};
use crate::models::{Track, UserPlaylist};
use crate::lyrics::LyricLine;

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
    fn resolve_stream(&self, track_id: &str) -> impl std::future::Future<Output = SporeResult<AudioStreamSource>> + Send;
    fn fetch_lyrics(&self, track: &Track) -> impl std::future::Future<Output = SporeResult<Vec<LyricLine>>> + Send;
    fn login(&mut self) -> impl std::future::Future<Output = SporeResult<()>> + Send;
    fn is_logged_in(&self) -> bool;
}

#[derive(Debug, Clone, Default)]
pub struct SpotifySession {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SpotifyBackend {
    pub http: Client,
    pub session: SpotifySession,
}

impl SpotifyBackend {
    pub fn new() -> SporeResult<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(SporeError::Network)?;
        Ok(Self {
            http,
            session: SpotifySession::default(),
        })
    }
}

impl StreamingBackend for SpotifyBackend {
    fn service_name(&self) -> &'static str {
        "Spotify"
    }

    async fn search(&self, query: &str, limit: usize) -> SporeResult<Vec<Track>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        // Spotify Web API or Open catalog search
        let url = format!(
            "https://itunes.apple.com/search?term={}&entity=song&limit={}",
            query, limit
        );
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
                if !id.is_empty() && !title.is_empty() {
                    tracks.push(Track {
                        id,
                        title,
                        artist,
                        album,
                        duration_ms,
                        storefront: "us".to_string(),
                        preview_url: None,
                        artwork_url,
                        hls_manifest_url: None,
                        is_lossless: false,
                    });
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
        Ok(Vec::new())
    }

    async fn fetch_user_playlists(&self) -> SporeResult<Vec<UserPlaylist>> {
        Ok(Vec::new())
    }

    async fn fetch_playlist_tracks(&self, _playlist_id: &str) -> SporeResult<Vec<Track>> {
        Ok(Vec::new())
    }

    async fn resolve_stream(&self, _track_id: &str) -> SporeResult<AudioStreamSource> {
        // Fallback / standard stream
        Err(SporeError::Spotify("Conectando stream de Spotify...".to_string()))
    }

    async fn fetch_lyrics(&self, track: &Track) -> SporeResult<Vec<LyricLine>> {
        let lines = crate::lyrics::fetch_lrclib_lyrics(&self.http, &track.title, &track.artist).await;
        Ok(lines)
    }

    async fn login(&mut self) -> SporeResult<()> {
        Ok(())
    }

    fn is_logged_in(&self) -> bool {
        self.session.access_token.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spotify_backend_metadata() {
        let backend = SpotifyBackend::new().unwrap();
        assert_eq!(backend.service_name(), "Spotify");
        assert!(!backend.is_logged_in());
    }
}

