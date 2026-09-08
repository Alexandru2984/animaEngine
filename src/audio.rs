//! One-shot sound playback with stereo panning by on-screen position.
//!
//! Behavior scripts call `play("meow.ogg")`; the sound is panned by where
//! the character is, so a mascot on the left of the desktop is heard on
//! the left. Files live under the asset library and resolve through the
//! same containment helper as sprites and scripts.
//!
//! # Everything here degrades to silence
//!
//! A missing audio device is normal, not exceptional — CI runners, most
//! VMs, and a machine with audio muted at the system level all have none.
//! `AudioHost::new` therefore cannot fail: it logs once and every later
//! call becomes a no-op. An overlay that refused to start because it could
//! not open ALSA would be a worse bug than silence.
//!
//! # Feature gate
//!
//! The module is always compiled; only the parts that touch `rodio` are
//! behind `feature = "audio"`. Keeping the type present unconditionally
//! means no caller's signature changes with the feature — a build without
//! audio gets a host whose `play` is a no-op, which is the same thing a
//! machine without a sound card gets anyway.
//!
//! # Why the caps exist
//!
//! A script runs sixty times a second and can call `play` on every one of
//! them. Without limits that is an unbounded decode, an unbounded mixer
//! queue, and a genuinely unpleasant noise. The per-sound cooldown and the
//! voice cap below are what make the feature safe to hand to a script.

#[cfg(feature = "audio")]
use std::collections::BTreeMap;
#[cfg(feature = "audio")]
use std::time::{Duration, Instant};

/// Largest sound file accepted. Mascot noises are short; this is the same
/// bound-the-read discipline every other loader here follows.
#[cfg(feature = "audio")]
const MAX_SOUND_BYTES: u64 = 4 * 1024 * 1024;
/// Cap on decoded audio held in memory across all cached sounds.
#[cfg(feature = "audio")]
const MAX_CACHED_BYTES: usize = 32 * 1024 * 1024;
/// Minimum gap between two plays of the same sound by the same entity.
///
/// A script calling `play` unconditionally each frame is the expected
/// mistake, not an exotic one. 150 ms still allows deliberate rapid
/// effects while making a per-frame call sound like one deliberate sound.
#[cfg(feature = "audio")]
const RETRIGGER_COOLDOWN: Duration = Duration::from_millis(150);
/// Sounds allowed to start within one tick, across the whole scene.
#[cfg(feature = "audio")]
const MAX_VOICES_PER_TICK: usize = 8;

/// A decoded sound, kept ready to play again without touching the disk.
#[cfg(feature = "audio")]
struct Cached {
    samples: Vec<f32>,
    // rodio models these as non-zero, which is worth keeping rather than
    // unwrapping into plain integers and re-validating later.
    channels: rodio::ChannelCount,
    sample_rate: rodio::SampleRate,
}

#[cfg(feature = "audio")]
impl Cached {
    fn bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<f32>()
    }
}

/// Owns the output device and the decoded-sound cache.
pub struct AudioHost {
    /// `None` when no device could be opened, and absent entirely on a
    /// build without the `audio` feature. Every play is then a no-op.
    #[cfg(feature = "audio")]
    sink: Option<rodio::MixerDeviceSink>,
    #[cfg(feature = "audio")]
    cache: BTreeMap<String, Cached>,
    #[cfg(feature = "audio")]
    cached_bytes: usize,
    /// Last time each (entity, sound) pair was triggered, for the
    /// retrigger cooldown.
    #[cfg(feature = "audio")]
    last_played: BTreeMap<(String, String), Instant>,
    /// Sounds refused this tick, so a broken script is reported once
    /// rather than sixty times a second — same reasoning as
    /// `ScriptHost::note_failure`.
    #[cfg(feature = "audio")]
    failed: BTreeMap<String, String>,
    unreported: Vec<(String, String)>,
    #[cfg(feature = "audio")]
    voices_this_tick: usize,
}

impl std::fmt::Debug for AudioHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioHost")
            .field("available", &self.is_available())
            .finish()
    }
}

impl Default for AudioHost {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioHost {
    /// Open the default output device, or fall back to silence.
    pub fn new() -> Self {
        #[cfg(feature = "audio")]
        let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(s) => Some(s),
            Err(e) => {
                // Info, not warn: having no audio device is an ordinary
                // configuration, not a fault to draw attention to.
                tracing::info!("No audio output device ({e}); sounds will be silent.");
                None
            }
        };
        Self {
            #[cfg(feature = "audio")]
            sink,
            #[cfg(feature = "audio")]
            cache: BTreeMap::new(),
            #[cfg(feature = "audio")]
            cached_bytes: 0,
            #[cfg(feature = "audio")]
            last_played: BTreeMap::new(),
            #[cfg(feature = "audio")]
            failed: BTreeMap::new(),
            unreported: Vec::new(),
            #[cfg(feature = "audio")]
            voices_this_tick: 0,
        }
    }

    /// Whether a device was opened. False means every play is silent —
    /// either no device, or a build without the `audio` feature.
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "audio")]
        {
            self.sink.is_some()
        }
        #[cfg(not(feature = "audio"))]
        {
            false
        }
    }

    /// Reset the per-tick voice budget. Called once per scene tick.
    pub fn begin_tick(&mut self) {
        #[cfg(feature = "audio")]
        {
            self.voices_this_tick = 0;
        }
    }

    /// Drain load failures the user hasn't been told about yet.
    pub fn take_new_failures(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.unreported)
    }

    /// Play `rel` for `entity_id`, panned for a character whose centre sits
    /// at `centre_x` within `[min_x, max_x]`.
    ///
    /// Silently does nothing when there is no device, when the same entity
    /// played the same sound within the cooldown, or when this tick's voice
    /// budget is spent. Load failures are reported once.
    #[allow(clippy::too_many_arguments)]
    pub fn play(
        &mut self,
        root: &std::path::Path,
        rel: &str,
        entity_id: &str,
        centre_x: f32,
        min_x: f32,
        max_x: f32,
    ) {
        #[cfg(not(feature = "audio"))]
        {
            let _ = (root, rel, entity_id, centre_x, min_x, max_x);
        }
        #[cfg(feature = "audio")]
        self.play_inner(root, rel, entity_id, centre_x, min_x, max_x);
    }

    #[cfg(feature = "audio")]
    #[allow(clippy::too_many_arguments)]
    fn play_inner(
        &mut self,
        root: &std::path::Path,
        rel: &str,
        entity_id: &str,
        centre_x: f32,
        min_x: f32,
        max_x: f32,
    ) {
        if self.sink.is_none() || rel.is_empty() {
            return;
        }
        // A sound already known to be unloadable is not retried; the entry
        // clears only on a fresh host.
        if self.failed.contains_key(rel) {
            return;
        }
        if self.voices_this_tick >= MAX_VOICES_PER_TICK {
            return;
        }

        let key = (entity_id.to_string(), rel.to_string());
        let now = Instant::now();
        if let Some(last) = self.last_played.get(&key) {
            if now.duration_since(*last) < RETRIGGER_COOLDOWN {
                return;
            }
        }

        if !self.cache.contains_key(rel) {
            match load_sound(root, rel) {
                Ok(sound) => {
                    // Evicting on a byte budget rather than a count: one
                    // long sound can outweigh many short ones.
                    if self.cached_bytes + sound.bytes() > MAX_CACHED_BYTES {
                        self.cache.clear();
                        self.cached_bytes = 0;
                    }
                    self.cached_bytes += sound.bytes();
                    self.cache.insert(rel.to_string(), sound);
                }
                Err(e) => {
                    tracing::warn!("sound {rel}: {e}");
                    self.failed.insert(rel.to_string(), e.clone());
                    self.unreported.push((rel.to_string(), e));
                    return;
                }
            }
        }

        let Some(sound) = self.cache.get(rel) else {
            return;
        };
        let Some(sink) = &self.sink else { return };

        let (left, right) = pan_gains(centre_x, min_x, max_x);
        let buffer = rodio::buffer::SamplesBuffer::new(
            sound.channels,
            sound.sample_rate,
            sound.samples.clone(),
        );
        sink.mixer()
            .add(rodio::source::ChannelVolume::new(buffer, vec![left, right]));

        self.last_played.insert(key, now);
        self.voices_this_tick += 1;
    }
}

/// Per-channel gains for a character centred at `centre_x`.
///
/// Equal-power rather than linear: a linear pan dips ~3 dB in the middle,
/// so a mascot walking across the desktop would sound quieter in the
/// centre of its own journey. Full hard-panning is also avoided — a sound
/// that vanishes entirely from one ear reads as a glitch, so each side
/// keeps a floor.
#[cfg(feature = "audio")]
fn pan_gains(centre_x: f32, min_x: f32, max_x: f32) -> (f32, f32) {
    /// Quietest either channel gets, as a fraction of full gain.
    const FLOOR: f32 = 0.15;

    let span = max_x - min_x;
    // A zero-width desktop is degenerate; centre it rather than divide.
    let t = if span.abs() < f32::EPSILON {
        0.5
    } else {
        ((centre_x - min_x) / span).clamp(0.0, 1.0)
    };

    let angle = t * std::f32::consts::FRAC_PI_2;
    let left = angle.cos();
    let right = angle.sin();
    (FLOOR + (1.0 - FLOOR) * left, FLOOR + (1.0 - FLOOR) * right)
}

/// Decode one sound from the library into memory.
#[cfg(feature = "audio")]
fn load_sound(root: &std::path::Path, rel: &str) -> Result<Cached, String> {
    use rodio::Source;

    let resolved = crate::drop_validate::resolve_library_asset(root, std::path::Path::new(rel))?;
    let meta = std::fs::metadata(&resolved).map_err(|e| format!("sound unreadable: {e}"))?;
    if !meta.is_file() {
        return Err("sound is not a regular file".into());
    }
    if meta.len() > MAX_SOUND_BYTES {
        return Err(format!("sound larger than {MAX_SOUND_BYTES} bytes"));
    }

    let file = std::fs::File::open(&resolved).map_err(|e| format!("sound unreadable: {e}"))?;
    let decoder = rodio::Decoder::try_from(file).map_err(|e| format!("cannot decode: {e}"))?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples: Vec<f32> = decoder.collect();

    if samples.is_empty() {
        return Err("sound decoded to no audio".into());
    }
    Ok(Cached {
        samples,
        channels,
        sample_rate,
    })
}

// The panning maths is only compiled with the feature, so its tests are
// too. CI runs both configurations, so this stays covered.
#[cfg(all(test, feature = "audio"))]
mod pan_tests {
    use super::*;

    #[test]
    fn a_centred_character_is_heard_equally() {
        let (l, r) = pan_gains(960.0, 0.0, 1920.0);
        assert!((l - r).abs() < 1e-5, "centre was not balanced: {l} vs {r}");
    }

    #[test]
    fn the_left_edge_favours_the_left_ear() {
        let (l, r) = pan_gains(0.0, 0.0, 1920.0);
        assert!(l > r, "left edge did not favour the left: {l} vs {r}");
    }

    #[test]
    fn the_right_edge_favours_the_right_ear() {
        let (l, r) = pan_gains(1920.0, 0.0, 1920.0);
        assert!(r > l, "right edge did not favour the right: {l} vs {r}");
    }

    /// A sound must never disappear completely from one side — that reads
    /// as a glitch rather than as position.
    #[test]
    fn neither_ear_is_ever_silent() {
        for x in [-500.0, 0.0, 960.0, 1920.0, 5000.0] {
            let (l, r) = pan_gains(x, 0.0, 1920.0);
            assert!(l > 0.1 && r > 0.1, "channel dropped out at x={x}: {l}, {r}");
        }
    }

    /// Equal-power: a linear pan would dip in the middle, making a mascot
    /// quieter halfway across the desktop than at either end.
    #[test]
    fn perceived_loudness_stays_even_across_the_desktop() {
        let power = |x: f32| {
            let (l, r) = pan_gains(x, 0.0, 1920.0);
            l * l + r * r
        };
        let centre = power(960.0);
        for x in [0.0, 480.0, 1440.0, 1920.0] {
            let ratio = power(x) / centre;
            assert!(
                (0.8..=1.25).contains(&ratio),
                "loudness swung to {ratio:.2}x at x={x}"
            );
        }
    }

    #[test]
    fn positions_outside_the_desktop_are_clamped() {
        assert_eq!(pan_gains(-1000.0, 0.0, 1920.0), pan_gains(0.0, 0.0, 1920.0));
        assert_eq!(
            pan_gains(9999.0, 0.0, 1920.0),
            pan_gains(1920.0, 0.0, 1920.0)
        );
    }

    /// A single-monitor setup reporting a zero-width desktop must not
    /// divide by zero.
    #[test]
    fn a_zero_width_desktop_centres_instead_of_dividing() {
        let (l, r) = pan_gains(100.0, 100.0, 100.0);
        assert!(l.is_finite() && r.is_finite());
        assert!((l - r).abs() < 1e-5);
    }
}

#[cfg(test)]
mod host_tests {
    use super::*;

    /// Constructing the host must never fail, because a machine with no
    /// sound card is an ordinary machine — and neither must a build
    /// without the feature.
    #[test]
    fn a_machine_without_audio_still_gets_a_host() {
        let host = AudioHost::new();
        // Either outcome is valid; what matters is that it did not panic
        // and that playing is safe either way.
        let _ = host.is_available();
    }

    #[test]
    fn playing_without_a_device_or_path_is_a_no_op() {
        let mut host = AudioHost::new();
        host.begin_tick();
        host.play(
            std::path::Path::new("/nonexistent"),
            "",
            "e1",
            0.0,
            0.0,
            1920.0,
        );
        assert!(host.take_new_failures().is_empty());
    }
}
