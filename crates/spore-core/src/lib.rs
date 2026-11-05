//! Core library for the Spore Spotify terminal music player.

pub mod audio;
pub mod audio_cache;
pub mod auth;
pub mod backend;
pub mod db;
pub mod error;
pub mod i18n;
pub mod lyrics;
pub mod models;
pub mod queue;
pub mod transliteration;

pub use audio::NativeAudioEngine;
pub use audio_cache::AudioCache;
pub use auth::{SpotifyTokens, PkceSession};
pub use backend::{AudioStreamSource, SectionType, SpotifyBackend, StreamingBackend};
pub use db::CatalogStore;
pub use error::{SporeError, SporeResult};
pub use i18n::Language;
pub use lyrics::{fetch_lrclib_lyrics, parse_lrc, LyricLine};
pub use models::{PlaybackStatus, Track, UserPlaylist};
pub use queue::{PlaybackQueue, RepeatMode};
pub use transliteration::transliterate;
