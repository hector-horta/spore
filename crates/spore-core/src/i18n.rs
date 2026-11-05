use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Es,
    En,
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Language::Es => serializer.serialize_str("es"),
            Language::En => serializer.serialize_str("en"),
        }
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "es" => Ok(Language::Es),
            "en" => Ok(Language::En),
            _ => Err(serde::de::Error::custom("Invalid language code")),
        }
    }
}

impl Language {
    pub fn toggle(self) -> Self { match self { Self::Es => Self::En, Self::En => Self::Es } }
    pub fn code(&self) -> &'static str { match self { Self::Es => "es", Self::En => "en" } }
    pub fn name(&self) -> &'static str { match self { Self::Es => "Español", Self::En => "English" } }
    pub fn detect_system() -> Self {
        for k in ["LANG", "LC_ALL", "LC_MESSAGES"] {
            if let Ok(v) = std::env::var(k) {
                if v.to_lowercase().starts_with("es") { return Self::Es; }
            }
        }
        Self::En
    }
    pub fn nav_home(&self) -> &'static str { match self { Self::Es => "[1 - Inicio]", Self::En => "[1 - Home]" } }
    pub fn nav_new(&self) -> &'static str { match self { Self::Es => "[2 - Novedades]", Self::En => "[2 - New]" } }
    pub fn nav_radio(&self) -> &'static str { "[3 - Radio]" }
    pub fn nav_library(&self) -> &'static str { match self { Self::Es => "[4 - Biblioteca]", Self::En => "[4 - Library]" } }
    pub fn nav_playlists(&self) -> &'static str { "[5 - Playlists]" }
    pub fn nav_search(&self) -> &'static str { match self { Self::Es => "[/ - Buscar]", Self::En => "[/ - Search]" } }
    pub fn nav_queue(&self) -> &'static str { match self { Self::Es => "[Tab - Cola]", Self::En => "[Tab - Queue]" } }
    pub fn nav_login(&self) -> &'static str { "[u - Login]" }
    pub fn nav_help(&self) -> &'static str { match self { Self::Es => "[? - Ayuda]", Self::En => "[? - Help]" } }
    pub fn nav_lang(&self) -> &'static str { match self { Self::Es => "[L - Idioma]", Self::En => "[L - Language]" } }

    pub fn search_placeholder(&self) -> &'static str { match self { Self::Es => " 🔍 Presiona '/' para buscar canciones, artistas o álbumes en Spotify...", Self::En => " 🔍 Press '/' to search songs, artists or albums on Spotify..." } }
    pub fn btn_play_list(&self) -> &'static str { match self { Self::Es => "[p - Reproducir Lista]", Self::En => "[p - Play Playlist]" } }
    pub fn btn_pause(&self) -> &'static str { match self { Self::Es => "[Espacio - Pausa]", Self::En => "[Space - Pause]" } }
    pub fn btn_prev_next(&self) -> &'static str { match self { Self::Es => "[< / > - Ant/Sig]", Self::En => "[< / > - Prev/Next]" } }
    pub fn btn_shuffle(&self) -> &'static str { match self { Self::Es => "[s - Aleatorio]", Self::En => "[s - Shuffle]" } }
    pub fn btn_repeat(&self) -> &'static str { match self { Self::Es => "[r - Repetir]", Self::En => "[r - Repeat]" } }
    pub fn btn_enqueue(&self) -> &'static str { match self { Self::Es => "[a - Encolar]", Self::En => "[a - Enqueue]" } }
    pub fn btn_next_in_queue(&self) -> &'static str { match self { Self::Es => "[n - A Continuación]", Self::En => "[n - Play Next]" } }
    pub fn btn_remove_track(&self) -> &'static str { match self { Self::Es => "[d - Quitar Pista]", Self::En => "[d - Remove Track]" } }
    pub fn btn_phonetic_dual(&self) -> &'static str { match self { Self::Es => " Fonética: Dual", Self::En => " Lyrics: Dual" } }
    pub fn btn_phonetic_only(&self) -> &'static str { match self { Self::Es => " Fonética: Romaji", Self::En => " Lyrics: Romaji" } }
    pub fn btn_phonetic_off(&self) -> &'static str { match self { Self::Es => " Fonética: Off", Self::En => " Lyrics: Off" } }
    pub fn btn_clear_all(&self) -> &'static str { match self { Self::Es => "[c - Limpiar Todo]", Self::En => "[c - Clear All]" } }
    pub fn status_playing(&self) -> &'static str { match self { Self::Es => "▶  Reproduciendo", Self::En => "▶  Playing" } }
    pub fn status_paused(&self) -> &'static str { match self { Self::Es => "⏸  En Pausa", Self::En => "⏸  Paused" } }
    pub fn status_buffering(&self) -> &'static str { match self { Self::Es => "⏳  Cargando...", Self::En => "⏳  Buffering..." } }
    pub fn status_stopped(&self) -> &'static str { match self { Self::Es => "⏹  Detenido", Self::En => "⏹  Stopped" } }
    pub fn title_lyrics(&self) -> &'static str { match self { Self::Es => " Letras ", Self::En => " Lyrics " } }
    pub fn title_player(&self) -> &'static str { match self { Self::Es => " Reproductor ", Self::En => " Player " } }
    pub fn title_cover(&self) -> &'static str { match self { Self::Es => " Portada ", Self::En => " Cover " } }
    pub fn no_active_playback(&self) -> &'static str { match self { Self::Es => " Sin reproducción activa", Self::En => " No active playback" } }
    pub fn no_lyrics(&self) -> &'static str { match self { Self::Es => "  ♪ Reproduce una canción en Spotify para ver su letra en vivo.", Self::En => "  ♪ Play a song on Spotify to see live lyrics." } }
}
