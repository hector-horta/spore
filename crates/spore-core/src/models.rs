use serde::{Deserialize, Serialize};
use crate::error::{SporeError, SporeResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub storefront: String,
    pub preview_url: Option<String>,
    pub artwork_url: Option<String>,
    pub hls_manifest_url: Option<String>,
    pub is_lossless: bool,
}

impl Track {
    pub fn validate(&self) -> SporeResult<()> {
        if self.id.trim().is_empty() || self.title.trim().is_empty() {
            return Err(SporeError::Validation(
                "Track ID and title must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn formatted_duration(&self) -> String {
        let total_secs = self.duration_ms / 1000;
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrmMode {
    FreePreview,
    WidevineL3Cenc,
    WidevineL3Cbcs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackStatus {
    Stopped,
    Playing,
    Paused,
    Buffering,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPlaylist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub track_count: Option<usize>,
}