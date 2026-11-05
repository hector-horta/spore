use spore_core::{
    CatalogStore, NativeAudioEngine, PlaybackQueue, PlaybackStatus, RepeatMode,
    SpotifyBackend, SporeResult, StreamingBackend, Track, UserPlaylist,
};
use ratatui::widgets::ListState;
use crate::artwork::TerminalCoverArt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Home,
    New,
    Radio,
    Library,
    Playlists,
    Queue,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhoneticMode {
    #[default]
    Dual,
    Phonetic,
    Original,
}

impl PhoneticMode {
    pub fn next(&self) -> Self {
        match self {
            Self::Dual => Self::Phonetic,
            Self::Phonetic => Self::Original,
            Self::Original => Self::Dual,
        }
    }
}

pub struct TuiApp {
    pub search_input: String,
    pub is_editing: bool,
    pub tracks: Vec<Track>,
    pub selected_index: usize,
    pub list_state: ListState,
    pub queue: PlaybackQueue,
    pub queue_state: ListState,
    pub playlists: Vec<UserPlaylist>,
    pub playlist_state: ListState,
    pub active_view: ActiveView,
    pub current_track: Option<Track>,
    pub current_artwork: Option<TerminalCoverArt>,
    pub play_icon: crate::artwork::TerminalIcon,
    pub pause_icon: crate::artwork::TerminalIcon,
    pub current_lyrics: Vec<spore_core::LyricLine>,
    pub status_message: String,
    pub language: spore_core::Language,
    pub phonetic_mode: PhoneticMode,
    pub volume: f32,
    pub should_quit: bool,
    pub(crate) backend: SpotifyBackend,
    pub(crate) store: CatalogStore,
    pub(crate) audio: NativeAudioEngine,
    pub(crate) bg_tx: tokio::sync::mpsc::UnboundedSender<crate::playback::BgEvent>,
    pub(crate) bg_rx: tokio::sync::mpsc::UnboundedReceiver<crate::playback::BgEvent>,
    pub(crate) play_task: Option<tokio::task::JoinHandle<()>>,
    pub(crate) audio_cache: spore_core::AudioCache,
    pub(crate) spectrum: crate::spectrum_smoother::SpectrumSmoother,
}

impl TuiApp {
    pub fn new() -> SporeResult<Self> {
        let backend = SpotifyBackend::new()?;
        let store = CatalogStore::open_path("spore_cache.db")?;
        let cached = store.search_tracks("", 50).unwrap_or_default();
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut queue_state = ListState::default();
        queue_state.select(Some(0));
        let (bg_tx, bg_rx) = tokio::sync::mpsc::unbounded_channel();

        let audio_cache = spore_core::AudioCache::new(2 * 1024 * 1024 * 1024);
        let cache_bg = audio_cache.clone();
        tokio::task::spawn_blocking(move || { cache_bg.evict_if_needed(); });

        Ok(Self {
            search_input: String::with_capacity(1024),
            is_editing: false,
            tracks: cached,
            selected_index: 0,
            playlists: Vec::new(),
            playlist_state: ListState::default(),
            list_state,
            queue: PlaybackQueue::new(),
            queue_state,
            active_view: ActiveView::Home,
            current_track: None,
            current_artwork: None,
            play_icon: crate::artwork::TerminalIcon::load_or_fallback(
                &["assets/icons/transparent-play.png", "assets/icons/play.png"],
                6, 6,
                [" █◣   ", " ██◣  ", " █◤   "],
                ratatui::style::Color::Green,
            ),
            pause_icon: crate::artwork::TerminalIcon::load_or_fallback(
                &["assets/icons/transparent - pause.png", "assets/icons/pause.png"],
                6, 6,
                [" █ █  ", " █ █  ", " █ █  "],
                ratatui::style::Color::Yellow,
            ),
            current_lyrics: Vec::new(),
            status_message: String::new(),
            language: spore_core::Language::detect_system(),
            phonetic_mode: PhoneticMode::default(),
            volume: 0.85,
            should_quit: false,
            backend,
            store,
            audio: NativeAudioEngine::new()?,
            bg_tx,
            bg_rx,
            play_task: None,
            audio_cache,
            spectrum: crate::spectrum_smoother::SpectrumSmoother::new(),
        })
    }

    pub fn selected_track(&self) -> Option<Track> {
        self.tracks.get(self.selected_index).cloned()
    }

    pub fn next_track(&mut self) {
        if self.tracks.is_empty() { return; }
        if self.selected_index + 1 < self.tracks.len() {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
        self.list_state.select(Some(self.selected_index));
    }

    pub fn previous_track(&mut self) {
        if self.tracks.is_empty() { return; }
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.tracks.len() - 1;
        }
        self.list_state.select(Some(self.selected_index));
    }

    pub fn toggle_language(&mut self) {
        self.language = self.language.toggle();
    }

    pub fn toggle_phonetic_mode(&mut self) {
        self.phonetic_mode = self.phonetic_mode.next();
    }

    pub fn toggle_shuffle(&mut self) {
        let is_shuffled = self.queue.toggle_shuffle();
        self.status_message = if is_shuffled { "🔀 Modo Aleatorio Activado".to_string() } else { "🔀 Modo Aleatorio Desactivado".to_string() };
    }

    pub fn toggle_repeat(&mut self) {
        let mode = self.queue.toggle_repeat();
        self.status_message = match mode {
            RepeatMode::Off => "🔁 Repetición Desactivada".to_string(),
            RepeatMode::All => "🔁 Repetir Toda la Cola".to_string(),
            RepeatMode::One => "🔂 Repetir Canción Actual".to_string(),
        };
    }

    pub fn toggle_view(&mut self) {
        self.active_view = match self.active_view {
            ActiveView::Queue => ActiveView::Home,
            _ => ActiveView::Queue,
        };
    }

    pub fn enqueue_selected_back(&mut self) {
        if let Some(track) = self.selected_track() {
            let title = track.title.clone();
            self.queue.enqueue_back(track);
            self.status_message = format!("➕ Encolada al final: {}", title);
        }
    }

    pub fn enqueue_selected_next(&mut self) {
        if let Some(track) = self.selected_track() {
            let title = track.title.clone();
            self.queue.enqueue_next(track);
            self.status_message = format!("⏭ Encolada a continuación: {}", title);
        }
    }

    pub fn remove_selected_from_queue(&mut self) {
        if let Some(idx) = self.queue_state.selected() {
            if let Some(removed) = self.queue.remove(idx) {
                self.status_message = format!("🗑 Eliminada de la cola: {}", removed.title);
                let new_len = self.queue.len();
                if new_len == 0 {
                    self.queue_state.select(None);
                } else if idx >= new_len {
                    self.queue_state.select(Some(new_len - 1));
                }
            }
        }
    }

    pub fn clear_current_list(&mut self) {
        match self.active_view {
            ActiveView::Queue => {
                self.queue.clear();
                self.queue_state.select(None);
                self.status_message = "🗑 Cola de reproducción vaciada.".to_string();
            }
            ActiveView::Home => {
                let _ = self.store.clear_all_tracks();
                self.tracks.clear();
                self.selected_index = 0;
                self.list_state.select(None);
                self.status_message = "🗑 Historial de Recientes limpiado.".to_string();
            }
            _ => {}
        }
    }

    pub async fn execute_search(&mut self) {
        let q = self.search_input.trim();
        if q.is_empty() { return; }
        self.status_message = format!("Buscando '{}' en Spotify...", q);
        match self.backend.search(q, 50).await {
            Ok(results) => {
                let _ = self.store.upsert_tracks(&results);
                self.tracks = results;
                self.selected_index = 0;
                self.list_state.select(if self.tracks.is_empty() { None } else { Some(0) });
                self.status_message = format!("{} resultados para '{}'", self.tracks.len(), q);
            }
            Err(e) => self.status_message = format!("Error de búsqueda: {e}"),
        }
    }

    pub async fn sync_user_auth(&mut self) {
        match self.backend.login().await {
            Ok(()) => self.status_message = "✅ Sesión de Spotify sincronizada con éxito.".to_string(),
            Err(e) => self.status_message = format!("Error login Spotify: {e}"),
        }
    }

    pub fn playback_progress(&self) -> (std::time::Duration, std::time::Duration) {
        self.audio.progress()
    }

    pub fn playback_status(&self) -> PlaybackStatus {
        self.audio.status()
    }

    /// Smoothed spectrum bars (0.0..=1.0) as of the last `tick_spectrum`.
    pub fn current_spectrum(&self) -> [f32; spore_core::spectrum::BANDS] {
        self.spectrum.bars()
    }

    /// Advances the spectrum smoother to the current playback position. Call once per frame.
    pub fn tick_spectrum(&mut self) {
        self.spectrum.update(self.audio.current_spectrum());
    }

    pub fn is_logged_in(&self) -> bool {
        self.backend.is_logged_in()
    }

    pub async fn toggle_pause(&mut self) {
        self.audio.toggle_pause();
    }

    pub async fn adjust_volume(&mut self, delta: f32) {
        self.volume = (self.volume + delta).clamp(0.0, 1.0);
        self.audio.set_volume(self.volume);
    }
}
