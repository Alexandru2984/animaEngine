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
use futures_lite::StreamExt;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use zbus::zvariant::{OwnedValue, Value};

/// Actions exposed as portal shortcuts — same set as the XGrabKey
/// backend (`super::GLOBAL_ACTIONS`); per-entity actions stay local.
const PORTAL_ACTIONS: &[Action] = &[
    Action::ToggleEditMode,
    Action::HideOverlay,
    Action::PauseAll,
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

/// Portal shortcut id for an action. Stable public identifiers —
/// they show up in the desktop's shortcut settings UI, so changing
/// one orphans the user's stored binding.
pub fn action_slug(action: Action) -> Option<&'static str> {
    Some(match action {
        Action::ToggleEditMode => "toggle-edit-mode",
        Action::HideOverlay => "hide-overlay",
        Action::PauseAll => "pause-all",
        _ => return None,
    })
}

/// Inverse of [`action_slug`].
pub fn slug_action(slug: &str) -> Option<Action> {
    Some(match slug {
        "toggle-edit-mode" => Action::ToggleEditMode,
        "hide-overlay" => Action::HideOverlay,
        "pause-all" => Action::PauseAll,
        _ => return None,
    })
}

/// Human description shown in the desktop's shortcut dialog. English
/// on purpose: the portal stores it at bind time, so a locale switch
/// in-app wouldn't update it anyway — better one stable string than
/// a stale translation.
fn action_description(action: Action) -> &'static str {
    match action {
        Action::ToggleEditMode => "Toggle edit mode",
        Action::HideOverlay => "Hide or show the overlay",
        Action::PauseAll => "Pause or resume all animations",
        _ => "",
    }
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

/// Unique, spec-legal handle token (`[A-Za-z0-9_]`). The portal uses
/// it to pre-compute the Request object path.
fn next_handle_token() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("anima_{}_{n}", std::process::id())
}

/// `:1.42` → `1_42`, per the portal Request-path convention.
fn sanitize_sender(unique_name: &str) -> String {
    unique_name
        .trim_start_matches(':')
        .replace('.', "_")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Extract a string entry from a portal response vardict.
fn vardict_str(results: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let v = results.get(key)?;
    match &**v {
        Value::Str(s) => Some(s.to_string()),
        Value::ObjectPath(p) => Some(p.to_string()),
        _ => None,
    }
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
            Some((
                slug.to_string(),
                action_description(action).to_string(),
                trigger,
            ))
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
            pump_activations(conn, shortcuts_proxy, session_handle, tx).await
        }
        Err(e) => {
            let _ = tx.try_send(PortalMsg::Failed);
            Err(e)
        }
    }
}

/// CreateSession + BindShortcuts. Returns the live connection, the
/// GlobalShortcuts proxy and the session handle for the signal pump.
async fn portal_handshake(
    rows: &[(String, String, Option<String>)],
) -> Result<(zbus::Connection, zbus::Proxy<'static>, String), String> {
    let conn = zbus::Connection::session()
        .await
        .map_err(|e| format!("session bus: {e}"))?;

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
    let results = portal_request(&conn, &sender, &shortcuts_proxy, "CreateSession", |token| {
        let mut opts: HashMap<&str, Value<'_>> = HashMap::new();
        opts.insert("handle_token", Value::from(token));
        opts.insert("session_handle_token", Value::from(session_token.clone()));
        (opts,)
    })
    .await?;

    let session_handle = vardict_str(&results, "session_handle")
        .ok_or("CreateSession response missing session_handle")?;
    tracing::debug!("Portal session: {session_handle}");

    // ── BindShortcuts ────────────────────────────────────────────────
    let results = portal_request(&conn, &sender, &shortcuts_proxy, "BindShortcuts", |token| {
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
    })
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
    }
    tracing::warn!("Portal Activated stream ended");
    Ok(())
}

/// One portal request round-trip: subscribe on the predicted Request
/// path, fire the method, await the `Response` signal, return its
/// results vardict. `build_args` receives the generated handle_token
/// by value so the argument tuple owns every string it serializes.
async fn portal_request<A>(
    conn: &zbus::Connection,
    sender: &str,
    proxy: &zbus::Proxy<'_>,
    method: &str,
    build_args: impl FnOnce(String) -> A,
) -> Result<HashMap<String, OwnedValue>, String>
where
    A: serde::Serialize + zbus::zvariant::DynamicType,
{
    let token = next_handle_token();
    let request_path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");

    let request_proxy = zbus::Proxy::new(
        conn,
        "org.freedesktop.portal.Desktop",
        request_path.as_str(),
        "org.freedesktop.portal.Request",
    )
    .await
    .map_err(|e| format!("Request proxy: {e}"))?;
    let mut responses = request_proxy
        .receive_signal("Response")
        .await
        .map_err(|e| format!("subscribe Response: {e}"))?;

    proxy
        .call_method(method, &build_args(token.clone()))
        .await
        .map_err(|e| format!("{method}: {e}"))?;

    let msg = responses
        .next()
        .await
        .ok_or_else(|| format!("{method}: response stream closed"))?;
    let (code, results) = msg
        .body()
        .deserialize::<(u32, HashMap<String, OwnedValue>)>()
        .map_err(|e| format!("{method} response decode: {e}"))?;
    match code {
        0 => Ok(results),
        1 => Err(format!("{method}: cancelled by user")),
        other => Err(format!("{method}: portal error code {other}")),
    }
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

    #[test]
    fn handle_tokens_are_unique_and_legal() {
        let a = next_handle_token();
        let b = next_handle_token();
        assert_ne!(a, b);
        for t in [&a, &b] {
            assert!(
                t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "token {t} carries an illegal char"
            );
        }
    }

    #[test]
    fn sender_sanitization_matches_portal_convention() {
        assert_eq!(sanitize_sender(":1.42"), "1_42");
        assert_eq!(sanitize_sender(":1.5-weird"), "1_5_weird");
    }

    #[test]
    fn vardict_reads_strings_and_object_paths() {
        let mut m: HashMap<String, OwnedValue> = HashMap::new();
        m.insert(
            "session_handle".into(),
            Value::from("/org/fdo/session/x").try_into().unwrap(),
        );
        assert_eq!(
            vardict_str(&m, "session_handle").as_deref(),
            Some("/org/fdo/session/x")
        );
        assert_eq!(vardict_str(&m, "missing"), None);
    }
}
