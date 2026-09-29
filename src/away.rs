//! Holding the scene still while nobody is there (1.5): after the idle
//! time the user picks, and — if they ask — while running on battery.
//!
//! Where the idle time comes from depends on the session:
//!
//! - native Wayland: `ext-idle-notify-v1`, event-driven
//!   (`wayland::layer_window::idle`);
//! - the X11 path on a real X server: the MIT-SCREEN-SAVER extension;
//! - the X11 path under a Wayland session (XWayland): Mutter's
//!   `org.gnome.Mutter.IdleMonitor` over D-Bus, so GNOME. XWayland's own
//!   screen-saver counter only sees input that reaches X11 windows: it
//!   would call the user away while they type into every native app, so
//!   it is never used there. KDE's D-Bus idle time is inconsistent about
//!   its units; under KDE on Wayland the X11 path has no source, and the
//!   setting says so.
//!
//! Battery comes from UPower's `OnBattery`, on the system bus.
//!
//! On the X11 path one thread watches both and sends [`AnimaEvent`]s when
//! they change; on native Wayland it watches the battery alone. The loops
//! tell it what to watch through [`configure`] — two process-wide atomics,
//! as there is only ever one watch.

use crate::event::{AnimaEvent, EventSink};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// The idle times offered, in minutes; 0 is never.
pub const IDLE_CHOICES: [u32; 5] = [0, 1, 5, 10, 30];

/// Idle time after which the scene holds still, in milliseconds; 0 off.
static IDLE_AFTER_MS: AtomicU64 = AtomicU64::new(0);
/// Whether to watch the battery at all.
static WATCH_BATTERY: AtomicBool = AtomicBool::new(false);

/// Tell the watch what the settings are now. Cheap; the loops call it
/// every frame.
pub fn configure(idle_minutes: u32, pause_on_battery: bool) {
    IDLE_AFTER_MS.store(u64::from(idle_minutes) * 60_000, Ordering::Relaxed);
    WATCH_BATTERY.store(pause_on_battery, Ordering::Relaxed);
}

/// Idle this long counts as away for reminders (`crate::reminders`) when
/// pausing is off: the time a break takes to count as one.
pub const REMINDER_AWAY_MINUTES: u32 = 5;

/// How long without input counts as away, in minutes; 0 not watched.
/// Pausing sets it; with pausing off, reminders still need to know — time
/// away is the break that starts them over — so they get their own.
pub fn idle_watch_minutes(pause_minutes: u32, has_reminders: bool) -> u32 {
    if pause_minutes > 0 {
        pause_minutes
    } else if has_reminders {
        REMINDER_AWAY_MINUTES
    } else {
        0
    }
}

/// How the scene holds still for someone being away, or on battery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stillness {
    /// Away: the characters doze off (`crate::doze`).
    pub doze: bool,
    /// On battery with someone there: the scene just stops.
    pub freeze: bool,
}

impl Stillness {
    /// Whether the scene holds still at all, one way or the other.
    pub fn any(self) -> bool {
        self.doze || self.freeze
    }
}

/// How the scene holds still, if it does: away past the idle time the
/// user picked, the characters doze off; on battery, if asked, it stops.
/// Away on battery, they doze.
pub fn stillness(
    idle_minutes: u32,
    away: bool,
    pause_on_battery: bool,
    on_battery: bool,
) -> Stillness {
    let doze = idle_minutes > 0 && away;
    Stillness {
        doze,
        freeze: pause_on_battery && on_battery && !doze,
    }
}

/// How often the thread looks.
const TICK: Duration = Duration::from_secs(1);
/// The battery changes slowly; look every this many ticks.
const BATTERY_EVERY: u32 = 10;

/// Watch the idle time (with `watch_idle`, the X11 path) and the battery,
/// sending [`AnimaEvent::Away`], [`AnimaEvent::OnBattery`] and — once the
/// idle source is known — [`AnimaEvent::IdleSource`] when they change.
pub fn spawn_watch(sink: EventSink, watch_idle: bool) {
    let spawned = std::thread::Builder::new()
        .name("anima-away".into())
        .spawn(move || {
            let mut idle = watch_idle.then(IdleSource::open);
            if let Some(source) = &idle {
                tracing::info!("Idle time from: {}", source.describe());
                sink.send(AnimaEvent::IdleSource(!matches!(source, IdleSource::None)));
            }
            let mut battery: Option<Battery> = None;
            let (mut away, mut on_battery) = (false, false);
            let mut watching_battery = false;
            let mut tick: u32 = 0;
            loop {
                let after = IDLE_AFTER_MS.load(Ordering::Relaxed);
                let now_away = match idle.as_mut() {
                    Some(source) if after > 0 => source.idle_ms().is_some_and(|ms| ms >= after),
                    _ => false,
                };
                // Resent until delivered: a full channel is not a gone loop.
                if now_away != away && sink.send(AnimaEvent::Away(now_away)) {
                    away = now_away;
                }
                // At once when the setting has just been turned on (or off),
                // then every so often.
                let watch = WATCH_BATTERY.load(Ordering::Relaxed);
                if tick.is_multiple_of(BATTERY_EVERY) || watch != watching_battery {
                    watching_battery = watch;
                    let now_on_battery = watch
                        && battery
                            .get_or_insert_with(Battery::open)
                            .on_battery()
                            .unwrap_or(false);
                    if now_on_battery != on_battery
                        && sink.send(AnimaEvent::OnBattery(now_on_battery))
                    {
                        on_battery = now_on_battery;
                    }
                }
                tick = tick.wrapping_add(1);
                std::thread::sleep(TICK);
            }
        });
    if let Err(e) = spawned {
        tracing::warn!("Away watch thread failed to start: {e}");
    }
}

/// Where the X11 path reads how long the user has been idle.
enum IdleSource {
    /// MIT-SCREEN-SAVER on a real X server.
    #[cfg(unix)]
    ScreenSaver {
        conn: Box<x11rb::rust_connection::RustConnection>,
        root: u32,
    },
    /// GNOME's IdleMonitor, under a Wayland session.
    #[cfg(unix)]
    Mutter(zbus::Connection),
    None,
}

impl IdleSource {
    fn open() -> Self {
        #[cfg(unix)]
        {
            use x11rb::connection::Connection as _;
            let x = x11rb::connect(None).ok();
            let xwayland = x.as_ref().is_some_and(|(conn, _)| is_xwayland(conn));
            if !xwayland {
                if let Some((conn, screen)) = x {
                    let root = conn.setup().roots[screen].root;
                    let mut source = Self::ScreenSaver {
                        conn: Box::new(conn),
                        root,
                    };
                    if source.idle_ms().is_some() {
                        return source;
                    }
                }
            }
            if let Ok(conn) = async_io::block_on(zbus::Connection::session()) {
                let mut source = Self::Mutter(conn);
                if source.idle_ms().is_some() {
                    return source;
                }
            }
        }
        Self::None
    }

    fn describe(&self) -> &'static str {
        match self {
            #[cfg(unix)]
            Self::ScreenSaver { .. } => "the X server (MIT-SCREEN-SAVER)",
            #[cfg(unix)]
            Self::Mutter(_) => "Mutter's IdleMonitor (D-Bus)",
            Self::None => "nowhere — pausing when away is unavailable",
        }
    }

    /// How long since the user's last input.
    fn idle_ms(&mut self) -> Option<u64> {
        match self {
            #[cfg(unix)]
            Self::ScreenSaver { conn, root } => {
                use x11rb::protocol::screensaver::ConnectionExt;
                let reply = conn.screensaver_query_info(*root).ok()?.reply().ok()?;
                Some(u64::from(reply.ms_since_user_input))
            }
            #[cfg(unix)]
            Self::Mutter(conn) => async_io::block_on(async {
                let reply = conn
                    .call_method(
                        Some("org.gnome.Mutter.IdleMonitor"),
                        "/org/gnome/Mutter/IdleMonitor/Core",
                        Some("org.gnome.Mutter.IdleMonitor"),
                        "GetIdletime",
                        &(),
                    )
                    .await
                    .ok()?;
                reply.body().deserialize::<u64>().ok()
            }),
            Self::None => None,
        }
    }
}

/// Whether the X server is XWayland, which advertises an extension by
/// that name.
#[cfg(unix)]
fn is_xwayland(conn: &impl x11rb::connection::RequestConnection) -> bool {
    conn.extension_information("XWAYLAND")
        .ok()
        .flatten()
        .is_some()
}

/// UPower on the system bus.
struct Battery {
    #[cfg(unix)]
    conn: Option<zbus::Connection>,
}

impl Battery {
    fn open() -> Self {
        Self {
            #[cfg(unix)]
            conn: async_io::block_on(zbus::Connection::system()).ok(),
        }
    }

    /// Whether the machine runs on battery; `None` without UPower.
    fn on_battery(&self) -> Option<bool> {
        #[cfg(unix)]
        {
            let conn = self.conn.as_ref()?;
            async_io::block_on(async {
                let reply = conn
                    .call_method(
                        Some("org.freedesktop.UPower"),
                        "/org/freedesktop/UPower",
                        Some("org.freedesktop.DBus.Properties"),
                        "Get",
                        &("org.freedesktop.UPower", "OnBattery"),
                    )
                    .await
                    .ok()?;
                let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
                bool::try_from(value).ok()
            })
        }
        #[cfg(not(unix))]
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn away_holds_still_only_when_asked_to() {
        assert!(
            !stillness(0, true, false, false).any(),
            "never: not even away"
        );
        assert!(stillness(10, true, false, false).doze);
        assert!(
            !stillness(10, false, false, true).any(),
            "on battery, not asked"
        );
        assert_eq!(
            stillness(0, false, true, true),
            Stillness {
                doze: false,
                freeze: true
            },
            "on battery with someone there: no Zzz"
        );
        assert!(!stillness(10, false, true, false).any());
        assert_eq!(
            stillness(10, true, true, true),
            Stillness {
                doze: true,
                freeze: false
            },
            "away on battery: asleep"
        );
    }

    #[test]
    fn reminders_are_told_about_time_away_even_with_pausing_off() {
        assert_eq!(idle_watch_minutes(10, true), 10, "pausing sets it");
        assert_eq!(idle_watch_minutes(0, true), REMINDER_AWAY_MINUTES);
        assert_eq!(idle_watch_minutes(0, false), 0, "nobody needs it");
        // Watched for reminders, it still holds nothing still.
        assert!(!stillness(0, true, false, false).any());
    }

    #[test]
    fn configuring_turns_minutes_into_milliseconds() {
        configure(5, true);
        assert_eq!(IDLE_AFTER_MS.load(Ordering::Relaxed), 300_000);
        assert!(WATCH_BATTERY.load(Ordering::Relaxed));
        configure(0, false);
        assert_eq!(IDLE_AFTER_MS.load(Ordering::Relaxed), 0);
    }
}
