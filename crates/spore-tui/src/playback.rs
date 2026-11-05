use spore_core::{AudioStreamSource, StreamingBackend, Track};
use crate::app::TuiApp;
use crate::artwork::TerminalCoverArt;

#[allow(dead_code)]
pub enum BgEvent {
    AudioReady(Track, Result<AudioStreamSource, String>, Vec<spore_core::LyricLine>, String),
    ArtworkReady(String, TerminalCoverArt),
    PlaylistReady(String, Result<Vec<Track>, String>),
}

impl TuiApp {
    pub async fn poll_bg_events(&mut self) {
        while let Ok(ev) = self.bg_rx.try_recv() {
            match ev {
                BgEvent::AudioReady(track, res, lyrics, hint) => {
                    if self.current_track.as_ref().map(|t| t.id == track.id).unwrap_or(true) {
                        self.current_lyrics = lyrics;
                        match res {
                            Ok(AudioStreamSource::RawEncoded { bytes, extension }) => {
                                match self.audio.play_encoded_bytes(bytes, extension) {
                                    Ok(()) => {
                                        self.audio.set_volume(self.volume);
                                        self.status_message = hint;
                                        self.current_track = Some(track);
                                        self.trigger_prefetch_next();
                                    }
                                    Err(e) => self.status_message = format!("Error de audio: {e}"),
                                }
                            }
                            Ok(AudioStreamSource::PcmDirect { samples, sample_rate, channels }) => {
                                match self.audio.play_pcm_samples(channels, sample_rate, samples) {
                                    Ok(()) => {
                                        self.audio.set_volume(self.volume);
                                        self.status_message = hint;
                                        self.current_track = Some(track);
                                        self.trigger_prefetch_next();
                                    }
                                    Err(e) => self.status_message = format!("Error de audio: {e}"),
                                }
                            }
                            Err(e) => self.status_message = e,
                        }
                    }
                }
                BgEvent::ArtworkReady(tid, art) => {
                    if self.current_track.as_ref().map(|t| t.id == tid).unwrap_or(true) {
                        self.current_artwork = Some(art);
                    }
                }
                BgEvent::PlaylistReady(name, Ok(list)) if !list.is_empty() => {
                    self.queue.clear();
                    for t in &list { self.queue.enqueue_back(t.clone()); }
                    self.tracks = list;
                    self.selected_index = 0;
                    self.list_state.select(Some(0));
                    self.active_view = crate::app::ActiveView::Library;
                    self.status_message = format!("Reproduciendo '{}' ({} pistas)...", name, self.queue.len());
                    self.play_next().await;
                }
                BgEvent::PlaylistReady(name, _) => {
                    self.status_message = format!("No se pudo cargar playlist '{name}'.");
                }
            }
        }
    }

    pub async fn play_track(&mut self, track: Track) {
        if let Some(old) = self.play_task.take() { old.abort(); }
        self.audio.stop();
        let orig_id = track.id.clone();
        let seed = format!("{}:{}:{}:{}", orig_id, track.artist, track.album, track.title);
        self.current_artwork = Some(TerminalCoverArt::from_jpeg_or_seed(None, &seed));
        self.current_lyrics = vec![spore_core::LyricLine::new(0, "⏳ Sincronizando letra...")];
        self.current_track = Some(track.clone());
        self.status_message = format!("⏳ Cargando audio: {} — {}...", track.artist, track.title);

        let tx = self.bg_tx.clone();
        let cache = self.audio_cache.clone();
        let backend = self.backend.clone();

        self.play_task = Some(tokio::spawn(async move {
            if let Some(cached_bytes) = cache.get(&orig_id) {
                let lyrics = backend.fetch_lyrics(&track).await.unwrap_or_default();
                let _ = tx.send(BgEvent::AudioReady(
                    track.clone(),
                    Ok(AudioStreamSource::RawEncoded { bytes: cached_bytes, extension: "ogg" }),
                    lyrics,
                    "[Spotify | Caché]".to_string(),
                ));
                return;
            }

            let stream_res = backend.resolve_stream(&orig_id).await;
            let lyrics = backend.fetch_lyrics(&track).await.unwrap_or_default();

            if let Ok(AudioStreamSource::RawEncoded { ref bytes, .. }) = stream_res {
                let _ = cache.put(&orig_id, bytes);
            }

            let hint = "[Spotify Ogg 320k]".to_string();
            let _ = tx.send(BgEvent::AudioReady(
                track.clone(),
                stream_res.map_err(|e| e.to_string()),
                lyrics,
                hint,
            ));
        }));
    }

    pub async fn play_selected(&mut self) {
        if self.active_view == crate::app::ActiveView::Playlists {
            if let Some(pl) = self.playlists.get(self.playlist_state.selected().unwrap_or(0)).cloned() {
                self.status_message = format!("⏳ Cargando playlist '{}'...", pl.name);
                match self.backend.fetch_playlist_tracks(&pl.id).await {
                    Ok(t) => {
                        self.tracks = t;
                        self.selected_index = 0;
                        self.list_state.select(Some(0));
                        self.active_view = crate::app::ActiveView::Library;
                        self.status_message = format!("Playlist '{}' cargada ({} canciones). Presiona 'p' para reproducir toda.", pl.name, self.tracks.len());
                    }
                    Err(e) => self.status_message = format!("Error cargando playlist: {e}"),
                }
            }
        } else if let Some(track) = self.selected_track() {
            self.play_track(track).await;
        }
    }

    pub async fn play_next(&mut self) {
        if let Some(next) = self.queue.next_track(self.current_track.clone()) {
            self.play_track(next).await;
        }
    }

    pub async fn play_previous(&mut self) {
        if let Some(prev) = self.queue.previous_track(self.current_track.clone()) {
            self.play_track(prev).await;
        }
    }

    pub async fn check_auto_advance(&mut self) {
        if self.audio.is_finished() { self.play_next().await; }
    }

    pub async fn play_all(&mut self) {
        if self.active_view == crate::app::ActiveView::Playlists {
            if let Some(pl) = self.playlists.get(self.playlist_state.selected().unwrap_or(0)).cloned() {
                self.status_message = format!("⏳ Cargando playlist '{}'...", pl.name);
                let tx = self.bg_tx.clone();
                let backend = self.backend.clone();
                tokio::spawn(async move {
                    let res = backend.fetch_playlist_tracks(&pl.id).await.map_err(|e| e.to_string());
                    let _ = tx.send(BgEvent::PlaylistReady(pl.name, res));
                });
            }
            return;
        }
        if !self.tracks.is_empty() {
            self.queue.clear();
            let start = self.selected_index.min(self.tracks.len() - 1);
            for t in self.tracks[start..].iter().chain(self.tracks[..start].iter()) {
                self.queue.enqueue_back(t.clone());
            }
            self.play_next().await;
        }
    }

    pub fn trigger_prefetch_next(&mut self) {
        let Some(next_track) = self.queue.peek_next().cloned() else { return; };
        if self.audio_cache.contains(&next_track.id) { return; }

        let cache = self.audio_cache.clone();
        let backend = self.backend.clone();
        tokio::spawn(async move {
            if let Ok(AudioStreamSource::RawEncoded { bytes, .. }) = backend.resolve_stream(&next_track.id).await {
                let _ = cache.put(&next_track.id, &bytes);
                cache.evict_if_needed();
            }
        });
    }
}
