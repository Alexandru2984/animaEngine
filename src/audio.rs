//! One-shot sound playback with stereo panning by on-screen position.
//!
//! Behavior scripts call `play("meow.ogg")`; the sound is panned by where
//! the character is, so a mascot on the left of the desktop is heard on
//! the left. Files live under the asset library and resolve through the
//! same containment helper as sprites and scripts.
//!
//! WAV, OGG/Vorbis, FLAC, MP3 and **MP4** all decode. The last of those
//! means `play("mascot.mp4")` plays the audio track of a video the user
//! already dropped in as a sprite — symphonia handles both the container
//! and AAC, so there is no demuxing code here at all.
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
/// Cap on the *decoded* size of one sound.
///
/// `MAX_SOUND_BYTES` bounds the file, which bounds nothing useful: audio
/// codecs compress, and silence compresses enormously. A 23 KB FLAC
/// decodes to 38 MB of `f32` — a ratio of about 1600:1 — so the file cap
/// alone allowed a multi-gigabyte allocation from a file that passed
/// every check. The decode is stopped at this many samples instead of
/// being run to completion and measured afterwards.
#[cfg(feature = "audio")]
const MAX_DECODED_SOUND_BYTES: usize = 8 * 1024 * 1024;
#[cfg(feature = "audio")]
const MAX_SOUND_SAMPLES: usize = MAX_DECODED_SOUND_BYTES / std::mem::size_of::<f32>();
/// Cap on decoded audio held in memory across all cached sounds.
#[cfg(feature = "audio")]
const MAX_CACHED_BYTES: usize = 32 * 1024 * 1024;
/// One sound must always fit in the cache on its own, or the eviction
/// below would clear everything and still overshoot the budget.
#[cfg(feature = "audio")]
const _: () = assert!(MAX_DECODED_SOUND_BYTES <= MAX_CACHED_BYTES);
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

/// A `rodio::Source` that reads a cached sound **in place**.
///
/// `SamplesBuffer` owns its `Vec<f32>`, so handing one to the mixer meant
/// copying the whole decoded sound on every play. The per-tick voice cap
/// limits how many sounds *start*, not how many are still running, so a
/// script triggering the cap every frame kept stacking full-size copies
/// of the same buffer for as long as each took to finish. Sharing the
/// `Arc` makes a play cost a refcount bump instead.
#[cfg(feature = "audio")]
struct SharedSamples {
    sound: std::sync::Arc<Cached>,
    pos: usize,
}

#[cfg(feature = "audio")]
impl Iterator for SharedSamples {
    type Item = rodio::Sample;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let s = self.sound.samples.get(self.pos).copied();
        if s.is_some() {
            self.pos += 1;
        }
        s
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.sound.samples.len().saturating_sub(self.pos);
        (left, Some(left))
    }
}

#[cfg(feature = "audio")]
impl rodio::Source for SharedSamples {
    // Mirrors `rodio::buffer::SamplesBuffer`: the *total* span, with
    // `Some(0)` once exhausted, rather than the remaining count.
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        if self.pos >= self.sound.samples.len() {
            Some(0)
        } else {
            Some(self.sound.samples.len())
        }
    }

    #[inline]
    fn channels(&self) -> rodio::ChannelCount {
        self.sound.channels
    }

    #[inline]
    fn sample_rate(&self) -> rodio::SampleRate {
        self.sound.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        // Both are `NonZero` in rodio 0.22, which is exactly why `Cached`
        // keeps them in that form rather than as plain integers — no
        // zero-guard is needed here.
        let frames = self.sound.samples.len() / self.sound.channels.get() as usize;
        Some(Duration::from_secs_f64(
            frames as f64 / f64::from(self.sound.sample_rate.get()),
        ))
    }
}

/// Owns the output device and the decoded-sound cache.
pub struct AudioHost {
    /// `None` when no device could be opened, and absent entirely on a
    /// build without the `audio` feature. Every play is then a no-op.
    #[cfg(feature = "audio")]
    sink: Option<rodio::MixerDeviceSink>,
    #[cfg(feature = "audio")]
    cache: BTreeMap<String, std::sync::Arc<Cached>>,
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
            Ok(mut s) => {
                // rodio prints "Dropping DeviceSink, audio playing through
                // this sink will stop" whenever the sink is dropped, and
                // recommends leaving it on — it catches a sink dropped by
                // accident while you still expect sound.
                //
                // That is not the shape here: the host is owned for the
                // whole process, so the only drop is at exit, where the
                // message is pure noise on the user's terminal every time
                // they quit.
                s.log_on_drop(false);
                let c = s.config();
                tracing::info!(
                    "Audio output: {} channel(s) at {} Hz",
                    c.channel_count(),
                    c.sample_rate()
                );
                Some(s)
            }
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
                    //
                    // The clear used to be followed by an unconditional
                    // insert, so a sound bigger than the whole budget was
                    // cached anyway and left `cached_bytes` over the cap
                    // for good. It cannot happen now — `load_sound`
                    // refuses anything past MAX_DECODED_SOUND_BYTES, and
                    // the const assert above pins that below the budget —
                    // so after a clear there is always room.
                    let bytes = sound.bytes();
                    if self.cached_bytes + bytes > MAX_CACHED_BYTES {
                        self.cache.clear();
                        self.cached_bytes = 0;
                    }
                    self.cached_bytes += bytes;
                    self.cache
                        .insert(rel.to_string(), std::sync::Arc::new(sound));
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
        tracing::debug!(
            "sound {rel}: x {centre_x:.0} in [{min_x:.0}, {max_x:.0}], gains L {left:.2} R {right:.2}"
        );
        // Shares the decoded buffer rather than copying it — see
        // `SharedSamples`.
        let buffer = SharedSamples {
            sound: std::sync::Arc::clone(sound),
            pos: 0,
        };
        sink.mixer().add(panned(buffer, left, right));

        self.last_played.insert(key, now);
        self.voices_this_tick += 1;
    }
}

/// `sound` as a stereo source at the given gains.
#[cfg(feature = "audio")]
fn panned(sound: SharedSamples, left: f32, right: f32) -> impl rodio::Source + Send + 'static {
    rodio::source::ChannelVolume::new(sound, vec![left, right])
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

    // `take` one past the cap so an oversized sound is *detected* without
    // ever being fully decoded. Collecting first and measuring afterwards
    // is what let a 23 KB FLAC allocate 38 MB, and the same trick on a
    // file at the 4 MB limit would have gone far further. It also caps
    // what `collect` can pre-allocate from the decoder's size hint.
    let samples: Vec<f32> = decoder.take(MAX_SOUND_SAMPLES + 1).collect();

    if samples.is_empty() {
        return Err("sound decoded to no audio".into());
    }
    if samples.len() > MAX_SOUND_SAMPLES {
        // Refused rather than truncated: a sound that stops halfway is a
        // confusing artefact, and the caller reports a failure to the user
        // once and then stops retrying it.
        return Err(format!(
            "sound decodes to more than {} MiB of audio",
            MAX_DECODED_SOUND_BYTES / (1024 * 1024)
        ));
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

    /// Energy per output channel of a mono tone played through a stereo
    /// mixer at the given gains — the path `play` takes, minus the device.
    fn mixed_energy(rate: u32, left: f32, right: f32) -> (f32, f32) {
        use rodio::Source;
        let samples: Vec<f32> = (0..rate / 10)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 0.5)
            .collect();
        let sound = std::sync::Arc::new(Cached {
            samples,
            channels: rodio::ChannelCount::new(1).unwrap(),
            sample_rate: rodio::SampleRate::new(rate).unwrap(),
        });
        let (mixer, out) = rodio::mixer::mixer(
            rodio::ChannelCount::new(2).unwrap(),
            rodio::SampleRate::new(44_100).unwrap(),
        );
        mixer.add(panned(SharedSamples { sound, pos: 0 }, left, right));
        assert_eq!(out.channels().get(), 2);
        let (mut l, mut r) = (0.0, 0.0);
        let frames: Vec<f32> = out.take(44_100).collect();
        for pair in frames.chunks_exact(2) {
            l += pair[0] * pair[0];
            r += pair[1] * pair[1];
        }
        (l, r)
    }

    /// The gains have to survive rodio's mixer, not just `pan_gains`: the
    /// tests above never ran the chain `play` hands them to, and a mono
    /// sound through a stereo mixer at a different rate is exactly where a
    /// channel-count or resampling slip would centre every sound.
    #[test]
    fn a_panned_sound_is_louder_on_its_side_after_mixing() {
        for rate in [22_050, 44_100, 48_000] {
            let (l, r) = mixed_energy(rate, 1.0, 0.15);
            assert!(
                l > 10.0 * r,
                "{rate} Hz: left {l} was not louder than right {r}"
            );
            let (l, r) = mixed_energy(rate, 0.15, 1.0);
            assert!(
                r > 10.0 * l,
                "{rate} Hz: right {r} was not louder than left {l}"
            );
        }
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

    /// Every format we claim to accept must actually decode.
    ///
    /// MP4 matters most: it means a script can play the audio track of a
    /// video the user already dropped in as a sprite, with no demuxing
    /// code of our own — symphonia handles both container and codec.
    /// Enabled by rodio's `mp4` feature (isomp4 + aac), which is easy to
    /// drop by accident when trimming features, hence a test rather than
    /// a note.
    #[cfg(feature = "audio")]
    #[test]
    fn every_advertised_sound_format_decodes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/audio");
        for name in ["tone.wav", "tone.ogg", "tone.mp4"] {
            let sound =
                load_sound(&root, name).unwrap_or_else(|e| panic!("{name} did not decode: {e}"));
            assert!(!sound.samples.is_empty(), "{name} decoded to silence");
            assert!(
                sound.samples.iter().all(|s| s.is_finite()),
                "{name} produced non-finite samples"
            );
        }
    }

    /// A small file can decode to an enormous buffer, so the file-size
    /// cap bounds nothing that matters.
    ///
    /// The fixture is 43 KB of FLAC that decodes to about 50 MB of `f32`
    /// — a ratio near 1200:1, and silence compresses better still. The
    /// old loader ran the decode to completion and measured afterwards,
    /// so this allocated all 50 MB before anything could object; a file
    /// at the 4 MB limit would have gone orders of magnitude further.
    /// Reported against 1.1.0.
    #[cfg(feature = "audio")]
    #[test]
    fn a_small_file_that_decodes_huge_is_refused() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/audio");
        let on_disk = std::fs::metadata(root.join("long_silence.flac"))
            .expect("fixture present")
            .len();
        assert!(
            on_disk < MAX_SOUND_BYTES,
            "fixture must pass the file-size check to exercise the decode cap"
        );

        let err = match load_sound(&root, "long_silence.flac") {
            Err(e) => e,
            Ok(sound) => panic!(
                "a {} MiB decode was accepted",
                sound.bytes() / (1024 * 1024)
            ),
        };
        assert!(
            err.contains("MiB of audio"),
            "wrong refusal for an oversized decode: {err}"
        );
    }

    // One sound always fitting the cache budget is what stops the
    // eviction path from clearing everything and still overshooting —
    // which is what it used to do, inserting the oversized sound anyway
    // and leaving `cached_bytes` above the cap for good. That is pinned
    // by the `const _: () = assert!(…)` next to the constants, which
    // fails the build rather than a test run, so there is deliberately no
    // runtime test for it here.

    /// The containment helper guards sounds exactly as it guards scripts
    /// and sprites — worth pinning, since this is a separate loader.
    #[cfg(feature = "audio")]
    #[test]
    fn a_sound_cannot_be_loaded_from_outside_the_library() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/audio");
        assert!(load_sound(&root, "../../Cargo.toml").is_err());
        assert!(load_sound(&root, "/etc/passwd").is_err());
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
