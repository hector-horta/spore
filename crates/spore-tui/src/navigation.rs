use crate::app::{TuiApp, ActiveView};
use spore_core::StreamingBackend;

impl TuiApp {
    /// Synchronizes the user's playlists with Spotify.
    pub async fn sync_playlists(&mut self) {
        self.active_view = ActiveView::Playlists;
        match self.backend.fetch_user_playlists().await {
            Ok(playlists) => {
                self.status_message = format!("Playlists sincronizadas: {} listas encontradas (Enter para abrir).", playlists.len());
                self.playlists = playlists;
                self.playlist_state.select(if self.playlists.is_empty() { None } else { Some(0) });
            }
            Err(e) => {
                self.status_message = format!("Error sincronizando playlists: {}", e);
            }
        }
    }

    /// Synchronizes the user's library of songs with Spotify.
    pub async fn sync_library(&mut self) {
        self.active_view = ActiveView::Library;
        match self.backend.fetch_user_library().await {
            Ok(songs) => {
                self.status_message = format!("Biblioteca sincronizada: {} canciones cargadas.", songs.len());
                if !songs.is_empty() {
                    self.tracks = songs;
                    self.selected_index = 0;
                    self.list_state.select(Some(0));
                }
            }
            Err(e) => {
                self.status_message = format!("Error sincronizando biblioteca: {}", e);
            }
        }
    }

    /// Opens the home view with default search results.
    pub fn open_home(&mut self) {
        self.tracks = self.store.search_tracks("", 50).unwrap_or_default();
        self.selected_index = 0;
        self.list_state.select(Some(0));
        self.active_view = ActiveView::Home;
        self.status_message = "1 — Recientes: Historial de búsquedas ('/').".to_string();
    }

    /// Opens a section query with the provided search parameters.
    pub async fn open_section_query(&mut self, view: ActiveView, query: &str, label: &str) {
        self.active_view = view;
        if let Ok(res) = self.backend.search(query, 25).await {
            self.tracks = res;
            self.selected_index = 0;
            self.list_state.select(Some(0));
            self.status_message = format!("{label} cargadas ({} pistas).", self.tracks.len());
        }
    }
}
