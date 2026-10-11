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
//! Nothing here can fail: when the device cannot be opened, that is logged
//! once and the sound is skipped. An overlay that refused to start because
//! it could not open ALSA would be a worse bug than silence.
//!
//! # The device is open only while there is sound
//!
//! `AudioHost::new` opens nothing. The device is opened by the first sound
//! and closed once nothing has played for `LINGER`. rodio cannot pause a
//! stream, so an open device plays silence without end, and a stream that
//! never stops keeps the sound server's sink awake: no power saving, and
//! on some hardware an audible hiss, from an app that may never make a
//! sound. The first sound after a quiet spell pays for the open.
//!
//! Only the *default* device is used. rodio's `open_default_sink` falls
//! back to every other output when the default fails, and on Linux those
//! are raw ALSA hardware: with the sound server out of reach, the overlay
//! opened a GPU's HDMI output directly, behind PipeWire's back, and held it
//! for as long as it ran. Silence is the better failure.
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
/// How long the device stays open after the last sound has finished.
///
/// Long enough that a script chirping every few seconds does not reopen
/// the device for each chirp; short enough that the sound server gets its
/// sink back soon after the scene goes quiet.
#[cfg(feature = "audio")]
const LINGER: Duration = Duration::from_secs(30);
/// How long after a failed open the next sound tries again — so a sound
/// server started, or restarted, after the overlay is picked up within
/// seconds, without a script that plays every frame retrying sixty times
/// a second.
#[cfg(feature = "audio")]
const RETRY_AFTER: Duration = Duration::from_secs(10);
/// How long an open stream may take nothing from the mixer before it is
/// taken for dead — see [`Pulse`].
#[cfg(feature = "audio")]
const STALLED_AFTER: Duration = Duration::from_secs(2);

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

    fn duration(&self) -> Duration {
        // Both are `NonZero` in rodio 0.22, which is exactly why `Cached`
        // keeps them in that form rather than as plain integers — no
        // zero-guard is needed here.
        let frames = self.samples.len() / self.channels.get() as usize;
        Duration::from_secs_f64(frames as f64 / f64::from(self.sample_rate.get()))
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
    /// The host's gain, read on every sample, so the volume slider and
    /// mute act on a sound already playing ([`AudioHost::set_level`]).
    gain: std::sync::Arc<std::sync::atomic::AtomicU32>,
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
        let gain = f32::from_bits(self.gain.load(std::sync::atomic::Ordering::Relaxed));
        s.map(|s| s * gain)
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
        Some(self.sound.duration())
    }
}

/// A silent source that never ends, put in the mixer when the device opens
/// to count what the device takes from it.
///
/// A stream whose sound server went away — PipeWire restarted, say — stops
/// asking for samples without a word: cpal reports no error, and every
/// sound after it played into nothing until the scene had been quiet for
/// [`LINGER`], which a script that plays often never is. Seen in the test
/// rig with a private PipeWire stopped and started again. With the count
/// standing still ([`Liveness`]), the host closes the stream, and the next
/// sound opens whatever device there is by then.
#[cfg(feature = "audio")]
struct Pulse {
    /// Bumped once every [`Pulse::EVERY`] samples taken.
    taken: std::sync::Arc<std::sync::atomic::AtomicU64>,
    channels: rodio::ChannelCount,
    sample_rate: rodio::SampleRate,
    n: u32,
}

#[cfg(feature = "audio")]
impl Pulse {
    const EVERY: u32 = 1024;
}

#[cfg(feature = "audio")]
impl Iterator for Pulse {
    type Item = rodio::Sample;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.n += 1;
        if self.n >= Self::EVERY {
            self.n = 0;
            self.taken
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        Some(0.0)
    }
}

#[cfg(feature = "audio")]
impl rodio::Source for Pulse {
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> rodio::ChannelCount {
        self.channels
    }

    #[inline]
    fn sample_rate(&self) -> rodio::SampleRate {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Why an open stream is closed.
#[cfg(feature = "audio")]
enum Closing {
    Failed,
    Stalled,
    Quiet,
}

/// Whether a [`Pulse`]'s count has stood still for [`STALLED_AFTER`].
#[cfg(feature = "audio")]
#[derive(Debug)]
struct Liveness {
    seen: u64,
    since: Instant,
}

#[cfg(feature = "audio")]
impl Liveness {
    fn new(now: Instant) -> Self {
        Self {
            seen: 0,
            since: now,
        }
    }

    /// `taken` is the pulse's count at `now`.
    fn stalled(&mut self, taken: u64, now: Instant) -> bool {
        if taken != self.seen {
            self.seen = taken;
            self.since = now;
            return false;
        }
        now.duration_since(self.since) >= STALLED_AFTER
    }
}

/// The output device's state. See "The device is open only while there
/// is sound" in the module docs.
#[cfg(feature = "audio")]
enum Output {
    /// Nothing open: no sound has played yet, or the scene went quiet.
    Closed,
    Open {
        sink: rodio::MixerDeviceSink,
        /// When the last sound still playing ends, plus [`LINGER`].
        close_at: Instant,
        /// Set from the audio thread when the stream fails — the device
        /// was unplugged, the sound server went away.
        failed: std::sync::Arc<std::sync::atomic::AtomicBool>,
        /// The [`Pulse`]'s count, and whether it still moves: a stream
        /// can also die without failing.
        taken: std::sync::Arc<std::sync::atomic::AtomicU64>,
        liveness: Liveness,
    },
    /// The last open failed; sounds are skipped until `retry_at`.
    Unavailable { retry_at: Instant },
}

/// Whether the sounds are muted, as the loops last set it — for the tray
/// and a global shortcut, which say and decide which way to go without
/// the config at hand.
static MUTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the sounds are muted now (`sounds_muted`).
pub fn muted() -> bool {
    MUTED.load(std::sync::atomic::Ordering::Relaxed)
}

/// The gain for a volume from 0 to 1: its square, nearer how loudness is
/// heard than the volume itself — half way down is about 12 dB quieter,
/// not 6.
pub fn gain_for(volume: f32) -> f32 {
    let v = if volume.is_finite() {
        volume.clamp(0.0, 1.0)
    } else {
        1.0
    };
    v * v
}

/// Owns the output device and the decoded-sound cache.
pub struct AudioHost {
    /// The gain every playing sound reads ([`AudioHost::set_level`]), as
    /// `f32` bits.
    #[cfg(feature = "audio")]
    gain: std::sync::Arc<std::sync::atomic::AtomicU32>,
    /// Muted, at no volume, or quiet for bedtime: nothing new plays.
    silent: bool,
    /// Absent entirely on a build without the `audio` feature, where every
    /// play is a no-op.
    #[cfg(feature = "audio")]
    output: Output,
    /// Whether the device's settings and a failure to open have been
    /// logged — each once per process, not on every reopen.
    #[cfg(feature = "audio")]
    logged_open: bool,
    #[cfg(feature = "audio")]
    logged_unavailable: bool,
    /// Whether a stream found dead ([`Liveness`]) has been logged.
    #[cfg(feature = "audio")]
    logged_stalled: bool,
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
            .field("open", &self.is_open())
            .finish()
    }
}

impl Default for AudioHost {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioHost {
    /// A host with nothing open yet: the device waits for the first sound.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "audio")]
            gain: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(1.0f32.to_bits())),
            silent: false,
            #[cfg(feature = "audio")]
            output: Output::Closed,
            #[cfg(feature = "audio")]
            logged_open: false,
            #[cfg(feature = "audio")]
            logged_unavailable: false,
            #[cfg(feature = "audio")]
            logged_stalled: false,
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

    /// Whether the output device is open right now. It opens on the first
    /// sound and closes once the scene is quiet, so a fresh host says no —
    /// as does a build without the `audio` feature, always.
    pub fn is_open(&self) -> bool {
        #[cfg(feature = "audio")]
        {
            matches!(self.output, Output::Open { .. })
        }
        #[cfg(not(feature = "audio"))]
        {
            false
        }
    }

    /// Once per scene tick, paused or not: reset the voice budget, and
    /// close the device once nothing has played for `LINGER`.
    pub fn begin_tick(&mut self) {
        #[cfg(feature = "audio")]
        {
            self.voices_this_tick = 0;
            let now = Instant::now();
            // A failed stream is dropped too, not kept, and so is one that
            // stopped taking sound: the next sound then opens whatever the
            // default device is by then.
            let why = match &mut self.output {
                Output::Open {
                    close_at,
                    failed,
                    taken,
                    liveness,
                    ..
                } => {
                    use std::sync::atomic::Ordering::Relaxed;
                    if failed.load(Relaxed) {
                        Some(Closing::Failed)
                    } else if liveness.stalled(taken.load(Relaxed), now) {
                        Some(Closing::Stalled)
                    } else if now >= *close_at {
                        Some(Closing::Quiet)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            let Some(why) = why else {
                return;
            };
            self.output = Output::Closed;
            match why {
                Closing::Stalled if !self.logged_stalled => {
                    self.logged_stalled = true;
                    tracing::warn!(
                        "Audio output stopped taking sound (the sound server went away?); \
                         it is reopened for the next sound."
                    );
                }
                Closing::Stalled => tracing::debug!("Audio output closed: it stopped taking sound"),
                Closing::Failed => tracing::debug!("Audio output closed after a stream error"),
                Closing::Quiet => {
                    tracing::debug!("Audio output closed: nothing has played for a while")
                }
            }
        }
    }

    /// How loud from now on, and whether at all: the loops call it every
    /// tick with `sound_volume`, and `muted` — `sounds_muted`, or bedtime
    /// when it is to be quiet. Sounds already playing follow at once; a
    /// silent host plays nothing new and opens no device.
    pub fn set_level(&mut self, volume: f32, muted: bool) {
        MUTED.store(muted, std::sync::atomic::Ordering::Relaxed);
        let gain = if muted { 0.0 } else { gain_for(volume) };
        self.silent = gain <= 0.0;
        #[cfg(feature = "audio")]
        self.gain
            .store(gain.to_bits(), std::sync::atomic::Ordering::Relaxed);
    }

    /// Drain load failures the user hasn't been told about yet.
    pub fn take_new_failures(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.unreported)
    }

    /// Play `rel` for `entity_id`, panned for a character whose centre sits
    /// at `centre_x` within `[min_x, max_x]`, at that character's `volume`
    /// (0 to 1, applied as [`gain_for`] under the global level).
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
        volume: f32,
    ) {
        #[cfg(not(feature = "audio"))]
        {
            let _ = (root, rel, entity_id, centre_x, min_x, max_x, volume);
        }
        #[cfg(feature = "audio")]
        self.play_inner(root, rel, entity_id, centre_x, min_x, max_x, volume);
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
        volume: f32,
    ) {
        let now = Instant::now();
        // Muted: nothing is decoded and no device is opened for it.
        if self.silent
            || rel.is_empty()
            || matches!(self.output, Output::Unavailable { retry_at } if now < retry_at)
        {
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

        let Some(sound) = self.cache.get(rel).map(std::sync::Arc::clone) else {
            return;
        };
        // Opened only now, once there is a sound that decoded: a script
        // naming a broken file never touches the device.
        self.ensure_open(now);
        let Output::Open { sink, close_at, .. } = &mut self.output else {
            return;
        };

        let (left, right) = pan_gains(centre_x, min_x, max_x);
        // The character's own volume, under the global one (`gain`).
        let own = gain_for(volume);
        let (left, right) = (left * own, right * own);
        tracing::debug!(
            "sound {rel}: x {centre_x:.0} in [{min_x:.0}, {max_x:.0}], gains L {left:.2} R {right:.2}"
        );
        // Shares the decoded buffer rather than copying it — see
        // `SharedSamples`.
        *close_at = (*close_at).max(now + sound.duration() + LINGER);
        let buffer = SharedSamples {
            sound,
            pos: 0,
            gain: std::sync::Arc::clone(&self.gain),
        };
        sink.mixer().add(panned(buffer, left, right));

        self.last_played.insert(key, now);
        self.voices_this_tick += 1;
    }
}

#[cfg(feature = "audio")]
impl AudioHost {
    /// Open the device unless it is open already, or failed too recently.
    fn ensure_open(&mut self, now: Instant) {
        match self.output {
            Output::Open { .. } => return,
            Output::Unavailable { retry_at } if now < retry_at => return,
            Output::Closed | Output::Unavailable { .. } => {}
        }
        let failed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        match open_default_output(std::sync::Arc::clone(&failed)) {
            Ok(sink) => {
                let c = sink.config();
                let taken = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
                sink.mixer().add(Pulse {
                    taken: std::sync::Arc::clone(&taken),
                    channels: c.channel_count(),
                    sample_rate: c.sample_rate(),
                    n: 0,
                });
                if !self.logged_open {
                    self.logged_open = true;
                    tracing::info!(
                        "Audio output: {} channel(s) at {} Hz",
                        c.channel_count(),
                        c.sample_rate()
                    );
                } else {
                    tracing::debug!("Audio output reopened");
                }
                self.output = Output::Open {
                    sink,
                    close_at: now + LINGER,
                    failed,
                    taken,
                    liveness: Liveness::new(now),
                };
            }
            Err(e) => {
                if !self.logged_unavailable {
                    self.logged_unavailable = true;
                    // Info, not warn: having no audio device is an
                    // ordinary configuration, not a fault to draw
                    // attention to.
                    tracing::info!("No audio output device ({e}); sounds will be silent.");
                } else {
                    tracing::debug!("Audio output still unavailable ({e})");
                }
                self.output = Output::Unavailable {
                    retry_at: now + RETRY_AFTER,
                };
            }
        }
    }
}

/// The default output device, and only that — see the module docs for
/// why rodio's fallback to other devices is not used. `failed` is set if
/// the stream breaks later.
#[cfg(feature = "audio")]
fn open_default_output(
    failed: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<rodio::MixerDeviceSink, rodio::DeviceSinkError> {
    // rodio's own error callback prints every error, and a stream whose
    // device is gone errors on every buffer — a hundred lines a second,
    // into a log that may be the journal. Here the first failure of a
    // stream is reported (once per process) and the host closes it.
    let on_error = move |e: rodio::cpal::StreamError| {
        static REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        use std::sync::atomic::Ordering::Relaxed;
        if !failed.swap(true, Relaxed) && !REPORTED.swap(true, Relaxed) {
            tracing::warn!("Audio output failed ({e}); it is reopened for the next sound.");
        }
    };
    let mut sink = rodio::DeviceSinkBuilder::from_default_device()?
        .with_error_callback(on_error)
        .open_sink_or_fallback()?;
    // rodio prints "Dropping DeviceSink, audio playing through this sink
    // will stop" whenever the sink is dropped, to catch one dropped by
    // accident. Here every drop is deliberate — the scene went quiet, or
    // the app is quitting — and the message would be noise on the user's
    // terminal each time.
    sink.log_on_drop(false);
    Ok(sink)
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
mod pulse_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    #[test]
    fn a_pulse_is_silent_and_counts_what_is_taken() {
        let taken = Arc::new(AtomicU64::new(0));
        let mut pulse = Pulse {
            taken: Arc::clone(&taken),
            channels: rodio::ChannelCount::new(2).unwrap(),
            sample_rate: rodio::SampleRate::new(48_000).unwrap(),
            n: 0,
        };
        for _ in 0..Pulse::EVERY * 3 {
            assert_eq!(pulse.next(), Some(0.0), "silence, and never done");
        }
        assert_eq!(taken.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn a_stream_is_dead_once_its_count_stands_still() {
        let t0 = Instant::now();
        let mut live = Liveness::new(t0);
        // Moving: never stalled, however long it runs.
        for k in 1..10u64 {
            assert!(!live.stalled(k, t0 + STALLED_AFTER * k as u32));
        }
        // Standing still: not at once, but past STALLED_AFTER.
        let last = t0 + STALLED_AFTER * 9;
        assert!(!live.stalled(9, last + STALLED_AFTER / 2));
        assert!(live.stalled(9, last + STALLED_AFTER));
    }
}

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
        mixed_energy_at(rate, left, right, 1.0)
    }

    /// As [`mixed_energy`], the host's gain at `gain`.
    fn mixed_energy_at(rate: u32, left: f32, right: f32, gain: f32) -> (f32, f32) {
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
        let gain = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(gain.to_bits()));
        mixer.add(panned(
            SharedSamples {
                sound,
                pos: 0,
                gain,
            },
            left,
            right,
        ));
        assert_eq!(out.channels().get(), 2);
        let (mut l, mut r) = (0.0, 0.0);
        let frames: Vec<f32> = out.take(44_100).collect();
        for [left, right] in frames.as_chunks::<2>().0 {
            l += left * left;
            r += right * right;
        }
        (l, r)
    }

    #[test]
    fn the_volume_scales_what_reaches_the_mixer() {
        let (l, r) = mixed_energy_at(44_100, 1.0, 1.0, 1.0);
        let (lh, rh) = mixed_energy_at(44_100, 1.0, 1.0, 0.5);
        // Half the amplitude, a quarter of the energy.
        assert!((lh / l - 0.25).abs() < 0.01, "{}", lh / l);
        assert!((rh / r - 0.25).abs() < 0.01, "{}", rh / r);
        let (l0, r0) = mixed_energy_at(44_100, 1.0, 1.0, 0.0);
        assert!(l0 == 0.0 && r0 == 0.0, "muted is silent");
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

    /// A host touches no device until a sound plays: an app that never
    /// makes a sound never holds a stream — and neither do these tests,
    /// which would otherwise open the sound card of whoever runs them.
    #[test]
    fn the_gain_follows_the_square_of_the_volume() {
        assert_eq!(gain_for(1.0), 1.0);
        assert_eq!(gain_for(0.5), 0.25);
        assert_eq!(gain_for(0.0), 0.0);
        assert_eq!(gain_for(2.0), 1.0, "held to full");
        assert_eq!(gain_for(-1.0), 0.0);
        assert_eq!(gain_for(f32::NAN), 1.0, "a broken value plays as it comes");
    }

    /// Muted or at no volume, nothing new plays — checked without playing,
    /// so a failing test cannot sound on the machine running it.
    #[test]
    fn muted_or_at_no_volume_the_host_is_silent() {
        let mut host = AudioHost::new();
        assert!(!host.silent);
        host.set_level(0.8, true);
        assert!(host.silent && muted(), "muted");
        host.set_level(0.0, false);
        assert!(host.silent && !muted(), "at no volume");
        host.set_level(0.5, false);
        assert!(!host.silent);
        #[cfg(feature = "audio")]
        assert_eq!(
            f32::from_bits(host.gain.load(std::sync::atomic::Ordering::Relaxed)),
            0.25
        );
        assert!(!host.is_open(), "and no device was touched");
    }

    #[test]
    fn a_new_host_opens_no_device() {
        let host = AudioHost::new();
        assert!(!host.is_open());
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
            1.0,
        );
        assert!(host.take_new_failures().is_empty());
        assert!(!host.is_open(), "an empty path must not open the device");
    }
}
