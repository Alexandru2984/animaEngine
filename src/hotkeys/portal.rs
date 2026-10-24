//! `org.freedesktop.portal.GlobalShortcuts` client (T.1).
//!
//! The portal is the only sanctioned way to receive global shortcuts
//! on GNOME/KDE Wayland sessions (no XGrabKey there) and the only one
//! that works from inside the Flatpak sandbox. Protocol shape:
//!
//! 1. `CreateSession` — every portal call returns an
//!    `org.freedesktop.portal.Request` object path; the actual result
//!    arrives as a `Response` signal on that path. The path is
//!    predictable from our unique bus name + a `handle_token`, so we
//!    subscribe *before* calling to close the race.
//! 2. `BindShortcuts(session, [(id, {description, preferred_trigger})], …)`
//!    — the desktop may show a permission/editor dialog. Bindings are
//!    remembered by the portal **per app-id**, so subsequent launches
//!    rebind silently. (No restore token in this portal — that's a
//!    ScreenCast concept.)
//! 3. `Activated` / `Deactivated` signals carry the shortcut id; we
//!    forward activations into a **bounded** channel, same rationale
//!    as the D-Bus activation service (`sync_channel(64)`, overflow
//!    dropped at the sender).
//!
//! Everything here is best-effort: any failure logs and returns
//! `None`, and the caller falls back to the next strategy (T.2).
//! NOTE: the dev machine (Ubuntu 24.04, portal backend GNOME 46)
//! does not expose this interface — the pure helpers below carry the
//! test load; the live path gets validated on Plasma 6 / GNOME 48
//! before the 0.6 release (see docs/plans/v0.6-platform.md, risks).

use crate::keybindings::{Action, KeyBindings, KeyChord, KeyCode, NamedKey, SymbolKey};
use crate::portal::{next_handle_token, portal_request, sanitize_sender, vardict_str};
use futures_lite::StreamExt;
use std::collections::HashMap;
use std::sync::mpsc;
use zbus::zvariant::{OwnedValue, Value};

/// Actions exposed as portal shortcuts — same set as the XGrabKey
/// backend (`super::GLOBAL_ACTIONS`); per-entity actions stay local.
const PORTAL_ACTIONS: &[Action] = &[
    Action::ToggleEditMode,
    Action::HideOverlay,
    Action::PauseAll,
    Action::ToggleFocus,
    Action::ToggleSnooze,
    Action::ToggleMute,
];

/// Messages from the portal thread. The handshake outcome arrives as
/// `Ready`/`Failed` so the caller can run a *deferred* fallback (the
/// permission dialog may sit on screen for minutes — nothing blocks
/// startup waiting for it); activations follow after `Ready`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortalMsg {
    /// Session created and shortcuts bound.
    Ready,
    /// The dance failed (no portal, denial, bus error) — fall back.
    Failed,
    /// A bound shortcut fired.
    Activated(Action),
}

/// Bounded queue between the portal signal thread and the consumer.
/// Same cap + drop-on-overflow semantics as the D-Bus activation
/// service in `single_instance.rs`.
const PORTAL_QUEUE_CAP: usize = 64;

/// How long a shortcuts request may go unanswered. A portal that takes
/// the call and never replies (a buggy or hung backend) would otherwise
/// block the hotkey thread for good, and the fallback would never run.
const SHORTCUTS_TIMEOUT: Option<std::time::Duration> = Some(std::time::Duration::from_secs(15));

/// The same for `BindShortcuts`, which GNOME answers only once the user
/// has allowed the shortcuts in its dialog — the first time, and again
/// whenever the app asks for one more. At 15 s, someone reading the
/// dialog lost the portal to the X11 grab, which holds only over X
/// windows on Wayland (1.5 asks for six where 0.9 asked for three). Ten
/// minutes; the Keybindings tab says it is waiting meanwhile.
const BIND_TIMEOUT: Option<std::time::Duration> = Some(std::time::Duration::from_secs(10 * 60));

/// Portal shortcut id for an action. Stable public identifiers —
/// they show up in the desktop's shortcut settings UI, so changing
/// one orphans the user's stored binding.
pub fn action_slug(action: Action) -> Option<&'static str> {
    Some(match action {
        Action::ToggleEditMode => "toggle-edit-mode",
        Action::HideOverlay => "hide-overlay",
        Action::PauseAll => "pause-all",
        Action::ToggleFocus => "toggle-focus",
        Action::ToggleSnooze => "toggle-snooze",
        Action::ToggleMute => "toggle-mute",
        _ => return None,
    })
}

/// Inverse of [`action_slug`].
pub fn slug_action(slug: &str) -> Option<Action> {
    Some(match slug {
        "toggle-edit-mode" => Action::ToggleEditMode,
        "hide-overlay" => Action::HideOverlay,
        "pause-all" => Action::PauseAll,
        "toggle-focus" => Action::ToggleFocus,
        "toggle-snooze" => Action::ToggleSnooze,
        "toggle-mute" => Action::ToggleMute,
        _ => return None,
    })
}

/// Human description shown in the desktop's shortcut dialog: the
/// Keybindings tab's own words for the action, in the app's language when
/// the shortcuts are bound. They were English on purpose — the portal
/// keeps what it is given, so a language changed in the app shows there
/// only from the next start — but English in a German desktop's settings
/// is worse than words a start behind.
fn action_description(action: Action) -> String {
    crate::i18n::t(action.i18n_key())
}

/// Convert a chord to the XDG shortcuts-spec trigger string the
/// portal accepts as `preferred_trigger`, e.g. `CTRL+SHIFT+a`.
/// Returns `None` for keys the spec can't express — the portal then
/// lets the user pick a trigger in the system dialog.
pub fn chord_to_xdg_trigger(chord: KeyChord) -> Option<String> {
    let mut parts: Vec<&str> = Vec::with_capacity(4);
    if chord.mods.ctrl() {
        parts.push("CTRL");
    }
    if chord.mods.shift() {
        parts.push("SHIFT");
    }
    if chord.mods.alt() {
        parts.push("ALT");
    }
    if chord.mods.sup() {
        parts.push("LOGO");
    }
    let key: String = match chord.key {
        KeyCode::Letter(c) => c.to_ascii_lowercase().to_string(),
        KeyCode::Digit(d) => d.to_string(),
        KeyCode::Named(n) => match n {
            NamedKey::Escape => "Escape".into(),
            NamedKey::Space => "space".into(),
            NamedKey::Tab => "Tab".into(),
            NamedKey::Enter => "Return".into(),
            NamedKey::Backspace => "BackSpace".into(),
            NamedKey::Delete => "Delete".into(),
            NamedKey::Home => "Home".into(),
            NamedKey::End => "End".into(),
            NamedKey::PageUp => "Page_Up".into(),
            NamedKey::PageDown => "Page_Down".into(),
            NamedKey::ArrowUp => "Up".into(),
            NamedKey::ArrowDown => "Down".into(),
            NamedKey::ArrowLeft => "Left".into(),
            NamedKey::ArrowRight => "Right".into(),
            // The keysym names: F1 to F12.
            f @ NamedKey::F(_) => KeyCode::Named(f).canonical_str(),
        },
        KeyCode::Symbol(s) => match s {
            SymbolKey::Plus => "plus".into(),
            SymbolKey::Minus => "minus".into(),
            SymbolKey::Equal => "equal".into(),
            SymbolKey::BracketLeft => "bracketleft".into(),
            SymbolKey::BracketRight => "bracketright".into(),
            SymbolKey::Backquote => "grave".into(),
        },
    };
    parts.push(&key);
    Some(parts.join("+"))
}

/// Spawn the portal client in the background. Returns the message
/// receiver immediately — startup never waits on the permission
/// dialog. The first message is `Ready` or `Failed`; the consumer
/// runs its fallback on `Failed`.
///
/// `bindings` provides preferred triggers; the portal is free to
/// override them and the user can re-map in system settings.
pub fn spawn_bg(bindings: &KeyBindings) -> mpsc::Receiver<PortalMsg> {
    let (tx, rx) = mpsc::sync_channel::<PortalMsg>(PORTAL_QUEUE_CAP);

    // Snapshot the (slug, description, trigger) rows on the caller's
    // thread — the portal thread only needs these strings.
    let rows: Vec<(String, String, Option<String>)> = PORTAL_ACTIONS
        .iter()
        .filter_map(|&action| {
            let slug = action_slug(action)?;
            let trigger = bindings
                .chords_for(action)
                .first()
                .and_then(|&c| chord_to_xdg_trigger(c));
            Some((slug.to_string(), action_description(action), trigger))
        })
        .collect();

    let spawned = std::thread::Builder::new()
        .name("anima-portal-shortcuts".into())
        .spawn(move || {
            async_io::block_on(async move {
                if let Err(e) = run_portal_session(rows, tx).await {
                    tracing::warn!("GlobalShortcuts portal session failed: {e}");
                }
                // On success run_portal_session only returns when the
                // signal stream ends (connection dropped at exit);
                // Failed was already sent on the error path inside.
            });
        });
    if spawned.is_err() {
        tracing::warn!("Couldn't spawn portal thread");
        // rx with a hung-up sender: consumer sees disconnect == Failed.
    }
    rx
}

/// The full session dance, then the signal pump. Sends `Ready` once
/// shortcuts are bound (then `Activated` per press) or `Failed` on
/// any handshake error; returns `Err` only for logging.
async fn run_portal_session(
    rows: Vec<(String, String, Option<String>)>,
    tx: mpsc::SyncSender<PortalMsg>,
) -> Result<(), String> {
    match portal_handshake(&rows).await {
        Ok((conn, shortcuts_proxy, session_handle)) => {
            let _ = tx.try_send(PortalMsg::Ready);
            crate::wayland::waker::wake();
            pump_activations(conn, shortcuts_proxy, session_handle, tx).await
        }
        Err(e) => {
            let _ = tx.try_send(PortalMsg::Failed);
            crate::wayland::waker::wake();
            Err(e)
        }
    }
}

/// CreateSession + BindShortcuts. Returns the live connection, the
/// GlobalShortcuts proxy and the session handle for the signal pump.
async fn portal_handshake(
    rows: &[(String, String, Option<String>)],
) -> Result<(zbus::Connection, zbus::Proxy<'static>, String), String> {
    let mut conn = zbus::Connection::session()
        .await
        .map_err(|e| format!("session bus: {e}"))?;
    // Who we are, first (`portal::register_host_app`). A portal without
    // the registry reads it from our systemd scope instead, on a
    // connection's first call: give the desktop the moment it takes to
    // put us there, and ask on a fresh connection.
    if crate::portal::register_host_app(&conn).await == crate::portal::Registered::NoRegistry {
        crate::portal::wait_for_app_scope(std::time::Duration::from_secs(2)).await;
        conn = zbus::Connection::session()
            .await
            .map_err(|e| format!("session bus: {e}"))?;
    }

    let unique = conn
        .unique_name()
        .ok_or("connection has no unique name")?
        .to_string();
    let sender = sanitize_sender(&unique);

    let shortcuts_proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
    )
    .await
    .map_err(|e| format!("GlobalShortcuts proxy: {e}"))?;

    // ── CreateSession ────────────────────────────────────────────────
    let session_token = next_handle_token();
    let results = portal_request(
        &conn,
        &sender,
        &shortcuts_proxy,
        "CreateSession",
        SHORTCUTS_TIMEOUT,
        |token| {
            let mut opts: HashMap<&str, Value<'_>> = HashMap::new();
            opts.insert("handle_token", Value::from(token));
            opts.insert("session_handle_token", Value::from(session_token.clone()));
            (opts,)
        },
    )
    .await?;

    let session_handle = vardict_str(&results, "session_handle")
        .ok_or("CreateSession response missing session_handle")?;
    tracing::debug!("Portal session: {session_handle}");

    // ── BindShortcuts ────────────────────────────────────────────────
    let results = portal_request(
        &conn,
        &sender,
        &shortcuts_proxy,
        "BindShortcuts",
        BIND_TIMEOUT,
        |token| {
            let shortcuts: Vec<(String, HashMap<&str, Value<'_>>)> = rows
                .iter()
                .map(|(slug, desc, trigger)| {
                    let mut m: HashMap<&str, Value<'_>> = HashMap::new();
                    m.insert("description", Value::from(desc.clone()));
                    if let Some(t) = trigger {
                        m.insert("preferred_trigger", Value::from(t.clone()));
                    }
                    (slug.clone(), m)
                })
                .collect();
            let mut opts: HashMap<&str, Value<'_>> = HashMap::new();
            opts.insert("handle_token", Value::from(token));
            (
                zbus::zvariant::ObjectPath::try_from(session_handle.clone())
                    .expect("portal returned a valid object path"),
                shortcuts,
                "", // parent_window: none — overlay isn't a normal toplevel
                opts,
            )
        },
    )
    .await?;
    tracing::info!(
        "Portal shortcuts bound ({} entries)",
        results.len().max(rows.len())
    );

    Ok((conn, shortcuts_proxy, session_handle))
}

/// Receive `Activated` signals for our session and forward them as
/// [`PortalMsg::Activated`] until the stream ends. `_conn` is held so
/// the bus connection (and therefore the session) stays alive.
async fn pump_activations(
    _conn: zbus::Connection,
    shortcuts_proxy: zbus::Proxy<'static>,
    session_handle: String,
    tx: mpsc::SyncSender<PortalMsg>,
) -> Result<(), String> {
    let mut activated = shortcuts_proxy
        .receive_signal("Activated")
        .await
        .map_err(|e| format!("subscribe Activated: {e}"))?;

    while let Some(msg) = activated.next().await {
        // Body: (o session_handle, s shortcut_id, t timestamp, a{sv})
        let Ok((handle, shortcut_id, _ts, _opts)) = msg.body().deserialize::<(
            zbus::zvariant::OwnedObjectPath,
            String,
            u64,
            HashMap<String, OwnedValue>,
        )>() else {
            tracing::debug!("Portal Activated: undecodable body, skipped");
            continue;
        };
        if handle.as_str() != session_handle {
            continue; // another session of ours? not today — skip
        }
        let Some(action) = slug_action(&shortcut_id) else {
            tracing::debug!("Portal Activated for unknown id {shortcut_id:?}");
            continue;
        };
        if tx.try_send(PortalMsg::Activated(action)).is_err() {
            tracing::debug!("Portal activation dropped (queue full)");
        }
        crate::wayland::waker::wake();
    }
    tracing::warn!("Portal Activated stream ended");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::ModifierMask;

    #[test]
    fn slug_round_trip_for_every_portal_action() {
        for &action in PORTAL_ACTIONS {
            let slug = action_slug(action).expect("portal action must have a slug");
            assert_eq!(slug_action(slug), Some(action), "round trip for {slug}");
        }
    }

    #[test]
    fn non_portal_actions_have_no_slug() {
        assert_eq!(action_slug(Action::NudgeUp), None);
        assert_eq!(slug_action("definitely-not-a-slug"), None);
    }

    #[test]
    fn xdg_trigger_for_default_chords() {
        let chord: KeyChord = "Ctrl+Shift+A".parse().unwrap();
        assert_eq!(chord_to_xdg_trigger(chord).as_deref(), Some("CTRL+SHIFT+a"));
        let chord: KeyChord = "Ctrl+Shift+H".parse().unwrap();
        assert_eq!(chord_to_xdg_trigger(chord).as_deref(), Some("CTRL+SHIFT+h"));
    }

    #[test]
    fn xdg_trigger_super_maps_to_logo() {
        let chord = KeyChord::new(ModifierMask::SUPER, KeyCode::Letter('K'));
        assert_eq!(chord_to_xdg_trigger(chord).as_deref(), Some("LOGO+k"));
    }
}
