use spore_core::{AudioStreamSource, NativeAudioEngine};
use spore_core::spectrum::{analyze, SpectrumTimeline};

/// Decoded PCM plus its precomputed spectrum, ready to hand to the audio engine.
pub struct PreparedAudio {
    pub channels: u16,
    pub sample_rate: u32,
    pub samples: Vec<i16>,
    pub spectrum: SpectrumTimeline,
}

/// CPU-heavy: decodes (if needed) and runs the FFT. Never call on the UI thread.
pub fn prepare(src: AudioStreamSource) -> Result<PreparedAudio, String> {
    let (channels, sample_rate, samples) = match src {
        AudioStreamSource::RawEncoded { bytes, extension } => {
            NativeAudioEngine::decode_encoded_to_pcm(bytes, extension).map_err(|e| e.to_string())?
        }
        AudioStreamSource::PcmDirect { samples, sample_rate, channels } => (channels, sample_rate, samples),
    };
    let spectrum = analyze(&samples, channels, sample_rate);
    Ok(PreparedAudio { channels, sample_rate, samples, spectrum })
}

/// Runs `prepare` on tokio's blocking pool so the UI thread never stalls.
pub async fn prepare_async(src: Result<AudioStreamSource, String>) -> Result<PreparedAudio, String> {
    let src = src?;
    tokio::task::spawn_blocking(move || prepare(src))
        .await
        .map_err(|e| format!("Audio prep task failed: {e}"))?
}
