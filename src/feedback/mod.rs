//! Audio and visual feedback for recording state
//!
//! Provides pleasant audio cues when recording starts/stops
//! and manages visual indicators.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Sound configuration for different events
#[derive(Debug, Clone)]
pub struct ToneConfig {
    /// Base frequency in Hz
    pub frequency: f32,
    /// Duration in milliseconds
    pub duration_ms: u32,
    /// Volume (0.0 - 1.0)
    pub volume: f32,
    /// Optional second frequency for chord/interval
    pub frequency2: Option<f32>,
    /// Fade in/out time in ms (prevents clicks)
    pub fade_ms: u32,
}

impl ToneConfig {
    /// Pleasant ascending two-tone for "start recording"
    /// Uses a perfect fifth interval (C5 -> G5) which sounds uplifting
    pub fn start_recording() -> Self {
        Self {
            frequency: 523.25,  // C5
            duration_ms: 100,
            volume: 0.3,
            frequency2: Some(783.99), // G5 (perfect fifth)
            fade_ms: 15,
        }
    }

    /// Pleasant descending tone for "stop recording"
    /// Single warm tone that descends slightly via envelope
    pub fn stop_recording() -> Self {
        Self {
            frequency: 659.25,  // E5
            duration_ms: 150,
            volume: 0.25,
            frequency2: Some(523.25), // C5 (resolving down)
            fade_ms: 20,
        }
    }

    /// Short subtle tick for "transcription complete"
    pub fn transcription_complete() -> Self {
        Self {
            frequency: 880.0,   // A5
            duration_ms: 80,
            volume: 0.2,
            frequency2: Some(1108.73), // C#6 (major third - happy)
            fade_ms: 10,
        }
    }

    /// Error sound - dissonant minor second
    pub fn error() -> Self {
        Self {
            frequency: 330.0,   // E4
            duration_ms: 200,
            volume: 0.25,
            frequency2: Some(349.23), // F4 (minor second - tense)
            fade_ms: 25,
        }
    }

    /// Cancel sound - quick descending
    pub fn cancel() -> Self {
        Self {
            frequency: 440.0,   // A4
            duration_ms: 60,
            volume: 0.2,
            frequency2: None,
            fade_ms: 10,
        }
    }
}

/// Play a tone with the given configuration
pub fn play_tone(config: ToneConfig) {
    std::thread::spawn(move || {
        if let Err(e) = play_tone_blocking(&config) {
            warn!("Failed to play feedback tone: {}", e);
        }
    });
}

/// Play transcription complete sound (non-blocking)
pub fn play_complete_sound() {
    debug!("Playing transcription complete sound");
    play_tone(ToneConfig::transcription_complete());
}

/// Play error sound (non-blocking)
pub fn play_error_sound() {
    debug!("Playing error sound");
    play_tone(ToneConfig::error());
}

/// Play cancel sound (non-blocking)
pub fn play_cancel_sound() {
    debug!("Playing cancel sound");
    play_tone(ToneConfig::cancel());
}

/// Play a two-part tone sequence (for start: rising, for stop: falling)
pub fn play_start_sequence() {
    std::thread::spawn(|| {
        // First tone
        let tone1 = ToneConfig {
            frequency: 523.25,  // C5
            duration_ms: 80,
            volume: 0.25,
            frequency2: None,
            fade_ms: 10,
        };
        let _ = play_tone_blocking(&tone1);

        std::thread::sleep(Duration::from_millis(30));

        // Second tone (higher)
        let tone2 = ToneConfig {
            frequency: 659.25,  // E5
            duration_ms: 100,
            volume: 0.3,
            frequency2: None,
            fade_ms: 15,
        };
        let _ = play_tone_blocking(&tone2);
    });
}

/// Play stop sequence (falling tones)
pub fn play_stop_sequence() {
    std::thread::spawn(|| {
        // First tone
        let tone1 = ToneConfig {
            frequency: 659.25,  // E5
            duration_ms: 80,
            volume: 0.25,
            frequency2: None,
            fade_ms: 10,
        };
        let _ = play_tone_blocking(&tone1);

        std::thread::sleep(Duration::from_millis(30));

        // Second tone (lower, longer - feels conclusive)
        let tone2 = ToneConfig {
            frequency: 523.25,  // C5
            duration_ms: 120,
            volume: 0.28,
            frequency2: None,
            fade_ms: 20,
        };
        let _ = play_tone_blocking(&tone2);
    });
}

/// Blocking tone playback implementation
fn play_tone_blocking(config: &ToneConfig) -> anyhow::Result<()> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No output device available"))?;

    let supported_config = device
        .supported_output_configs()?
        .filter(|c| c.channels() == 2 || c.channels() == 1)
        .filter(|c| c.sample_format() == SampleFormat::F32)
        .next()
        .ok_or_else(|| anyhow::anyhow!("No suitable output config"))?;

    let sample_rate = if supported_config.min_sample_rate().0 <= 44100
        && supported_config.max_sample_rate().0 >= 44100
    {
        SampleRate(44100)
    } else {
        supported_config.min_sample_rate()
    };

    let stream_config = supported_config.with_sample_rate(sample_rate);
    let channels = stream_config.channels() as usize;
    let sr = sample_rate.0 as f32;

    let total_samples = (sr * config.duration_ms as f32 / 1000.0) as usize;
    let fade_samples = (sr * config.fade_ms as f32 / 1000.0) as usize;

    let freq1 = config.frequency;
    let freq2 = config.frequency2;
    let volume = config.volume;

    let sample_idx = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = Arc::clone(&done);
    let sample_idx_clone = Arc::clone(&sample_idx);

    let stream = device.build_output_stream(
        &stream_config.into(),
        move |data: &mut [f32], _: &_| {
            for frame in data.chunks_mut(channels) {
                let idx = sample_idx_clone.fetch_add(1, Ordering::Relaxed);

                if idx >= total_samples {
                    for sample in frame.iter_mut() {
                        *sample = 0.0;
                    }
                    done_clone.store(true, Ordering::Relaxed);
                    continue;
                }

                let t = idx as f32 / sr;

                // Generate sine wave(s)
                let mut value = (2.0 * PI * freq1 * t).sin();

                if let Some(f2) = freq2 {
                    // Add second frequency at slightly lower volume for richness
                    value += (2.0 * PI * f2 * t).sin() * 0.7;
                    value /= 1.7; // Normalize
                }

                // Apply envelope (fade in/out)
                let envelope = if idx < fade_samples {
                    // Fade in (smooth cosine curve)
                    0.5 * (1.0 - (PI * idx as f32 / fade_samples as f32).cos())
                } else if idx > total_samples - fade_samples {
                    // Fade out
                    let fade_idx = idx - (total_samples - fade_samples);
                    0.5 * (1.0 + (PI * fade_idx as f32 / fade_samples as f32).cos())
                } else {
                    1.0
                };

                let output = value * volume * envelope;

                for sample in frame.iter_mut() {
                    *sample = output;
                }
            }
        },
        |err| error!("Audio output error: {}", err),
        None,
    )?;

    stream.play()?;

    // Wait for playback to complete
    while !done.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(5));
    }

    // Small extra delay for audio to flush
    std::thread::sleep(Duration::from_millis(10));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires audio output
    fn test_start_sound() {
        play_start_sequence();
        std::thread::sleep(Duration::from_millis(500));
    }

    #[test]
    #[ignore] // Requires audio output
    fn test_stop_sound() {
        play_stop_sequence();
        std::thread::sleep(Duration::from_millis(500));
    }
}
