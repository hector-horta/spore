use std::io::Cursor;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::Mutex;
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    errors::Error as SymphoniaError,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use tracing::info;
use crate::error::{SporeError, SporeResult};
use crate::models::PlaybackStatus;

const STATE_STOPPED: u8 = 0;
const STATE_PLAYING: u8 = 1;
const STATE_PAUSED: u8 = 2;

pub struct NativeAudioEngine {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    sink: Mutex<Option<Sink>>,
    volume: Mutex<f32>,
    duration_secs: AtomicU64,
    started_at: Mutex<Option<std::time::Instant>>,
    accumulated_ms: AtomicU64,
    state: AtomicU8,
    spectrum_frames: Mutex<Vec<[u8; 32]>>,
}

impl NativeAudioEngine {
    pub fn new() -> SporeResult<Self> {
        let (_stream, handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => { info!("Audio output device unavailable ({e}), running in headless mode"); (None, None) }
        };
        Ok(Self {
            _stream,
            handle,
            sink: Mutex::new(None),
            volume: Mutex::new(0.85),
            duration_secs: AtomicU64::new(0),
            started_at: Mutex::new(None),
            accumulated_ms: AtomicU64::new(0),
            state: AtomicU8::new(STATE_STOPPED),
            spectrum_frames: Mutex::new(Vec::with_capacity(128)),
        })
    }

    pub fn decode_encoded_to_pcm(bytes: Vec<u8>, hint_ext: &str) -> SporeResult<(u16, u32, Vec<i16>)> {
        if bytes.is_empty() {
            return Err(SporeError::Audio("Empty audio buffer".to_string()));
        }

        let cursor = Cursor::new(bytes);
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

        let mut hint = Hint::new();
        hint.with_extension(hint_ext);

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| SporeError::Audio(format!("Probe error: {e}")))?;

        let mut format = probed.format;
        let track = format.tracks().iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| SporeError::Audio("No playable audio track found".to_string()))?;

        let track_id = track.id;
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| SporeError::Audio(format!("Decoder init error: {e}")))?;

        let mut pcm_samples: Vec<i16> = Vec::new();
        let mut sample_rate = 44100;
        let mut channels = 2;

        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track_id { continue; }
            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    let spec = *audio_buf.spec();
                    sample_rate = spec.rate;
                    channels = spec.channels.count() as u16;
                    let mut sbuf = SampleBuffer::<i16>::new(audio_buf.capacity() as u64, spec);
                    sbuf.copy_interleaved_ref(audio_buf);
                    pcm_samples.extend_from_slice(sbuf.samples());
                }
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(SymphoniaError::IoError(_)) => break,
                Err(e) => return Err(SporeError::Audio(format!("Decode packet error: {e}"))),
            }
        }

        if pcm_samples.is_empty() {
            return Err(SporeError::Audio("Decoded PCM audio stream is empty".to_string()));
        }

        Ok((channels, sample_rate, pcm_samples))
    }

    pub fn play_pcm_samples(&self, channels: u16, sample_rate: u32, pcm_samples: Vec<i16>) -> SporeResult<()> {
        let total_samples = pcm_samples.len();
        let dur = total_samples as u64 / (sample_rate as u64 * channels as u64);
        self.duration_secs.store(dur, Ordering::SeqCst);
        self.accumulated_ms.store(0, Ordering::SeqCst);
        *self.started_at.lock().unwrap() = Some(std::time::Instant::now());
        self.state.store(STATE_PLAYING, Ordering::SeqCst);

        // Precompute spectrum frames
        let mut frames = Vec::new();
        let chunk_size = (sample_rate as usize * channels as usize / 10).max(1); // 100ms chunks
        for chunk in pcm_samples.chunks(chunk_size) {
            let mut frame = [0u8; 32];
            for (i, sample) in chunk.iter().step_by(chunk.len() / 32 + 1).enumerate().take(32) {
                let amp = (sample.unsigned_abs() >> 8) as u8;
                frame[i] = amp.min(100);
            }
            frames.push(frame);
        }
        *self.spectrum_frames.lock().unwrap() = frames;

        if let Some(handle) = &self.handle {
            let sink = Sink::try_new(handle).map_err(|e| SporeError::Audio(e.to_string()))?;
            sink.set_volume(*self.volume.lock().unwrap());
            let source = SamplesBuffer::new(channels, sample_rate, pcm_samples);
            sink.append(source);
            sink.play();
            *self.sink.lock().unwrap() = Some(sink);
        }

        Ok(())
    }

    pub fn play_encoded_bytes(&self, bytes: Vec<u8>, format: &str) -> SporeResult<()> {
        let (channels, rate, samples) = Self::decode_encoded_to_pcm(bytes, format)?;
        self.play_pcm_samples(channels, rate, samples)
    }

    pub fn pause(&self) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.pause();
        }
        if self.state.load(Ordering::SeqCst) == STATE_PLAYING {
            if let Some(started) = self.started_at.lock().unwrap().take() {
                let elapsed = started.elapsed().as_millis() as u64;
                self.accumulated_ms.fetch_add(elapsed, Ordering::SeqCst);
            }
            self.state.store(STATE_PAUSED, Ordering::SeqCst);
        }
    }

    pub fn resume(&self) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.play();
        }
        if self.state.load(Ordering::SeqCst) == STATE_PAUSED {
            *self.started_at.lock().unwrap() = Some(std::time::Instant::now());
            self.state.store(STATE_PLAYING, Ordering::SeqCst);
        }
    }

    pub fn stop(&self) {
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
        self.state.store(STATE_STOPPED, Ordering::SeqCst);
        *self.started_at.lock().unwrap() = None;
        self.accumulated_ms.store(0, Ordering::SeqCst);
        self.spectrum_frames.lock().unwrap().clear();
    }

    pub fn set_volume(&self, vol: f32) {
        let vol = vol.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = vol;
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.set_volume(vol);
        }
    }

    pub fn status(&self) -> PlaybackStatus {
        match self.state.load(Ordering::SeqCst) {
            STATE_PLAYING => PlaybackStatus::Playing,
            STATE_PAUSED => PlaybackStatus::Paused,
            _ => PlaybackStatus::Stopped,
        }
    }

    pub fn progress_secs(&self) -> u64 {
        let acc = self.accumulated_ms.load(Ordering::SeqCst);
        let extra = if self.state.load(Ordering::SeqCst) == STATE_PLAYING {
            self.started_at.lock().unwrap().map(|t| t.elapsed().as_millis() as u64).unwrap_or(0)
        } else {
            0
        };
        (acc + extra) / 1000
    }

    pub fn duration_secs(&self) -> u64 {
        self.duration_secs.load(Ordering::SeqCst)
    }

    pub fn current_spectrum_frame(&self) -> [u8; 32] {
        let frames = self.spectrum_frames.lock().unwrap();
        if frames.is_empty() { return [0u8; 32]; }
        let progress_ms = self.progress_secs() * 1000;
        let idx = (progress_ms / 100) as usize;
        frames.get(idx).copied().unwrap_or_else(|| *frames.last().unwrap_or(&[0u8; 32]))
    }
    pub fn is_finished(&self) -> bool {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.empty()
        } else {
            false
        }
    }

    pub fn toggle_pause(&self) {
        if self.status() == PlaybackStatus::Playing {
            self.pause();
        } else if self.status() == PlaybackStatus::Paused {
            self.resume();
        }
    }

    pub fn progress(&self) -> (std::time::Duration, std::time::Duration) {
        (
            std::time::Duration::from_secs(self.progress_secs()),
            std::time::Duration::from_secs(self.duration_secs()),
        )
    }

    pub fn spectrum_bands_48(&self) -> [f32; 48] {
        let f = self.current_spectrum_frame();
        let mut out = [0.0f32; 48];
        for i in 0..48 {
            out[i] = f[(i * 32) / 48] as f32;
        }
        out
    }
}
