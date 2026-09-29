//! Starting with the session (1.5): the Appearance switch that has the
//! desktop start animaEngine at login.
//!
//! - Installed natively — a package, an AppImage, a build — it is an XDG
//!   autostart entry, `~/.config/autostart/com.animaengine.Anima.desktop`.
//!   The file is the truth: the desktop's own "Startup Applications" edits
//!   the same folder, so the switch reads the file rather than a setting.
//! - In the Flatpak the sandbox can neither write nor read that folder.
//!   The Background portal's `RequestBackground` asks the desktop to add
//!   or remove the entry, perhaps asking the user first; its answer is
//!   kept as `start_at_login` in the config, all the app can know of it.
//! - Not on Windows: there is no switch there.

use crate::error::{AnimaError, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};

/// The entry's file name: the app id, as the portal names it too.
pub const ENTRY_NAME: &str = "com.animaengine.Anima.desktop";

/// Whether this process runs inside a Flatpak.
pub fn in_flatpak() -> bool {
    Path::new("/.flatpak-info").exists()
}

/// Where the native entry goes.
pub fn entry_path() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.config_dir().join("autostart").join(ENTRY_NAME))
}

/// Whether the app starts with the session: the entry on disk when
/// installed natively; in the Flatpak, what the portal last granted
/// (`granted`, from the config).
pub fn is_enabled(granted: bool) -> bool {
    if in_flatpak() {
        granted
    } else {
        entry_path().is_some_and(|p| p.exists())
    }
}

/// Turn it on or off. Natively the entry is written or removed at once;
/// in the Flatpak the portal is asked on a thread of its own, and its
/// answer comes back through [`take_portal_answer`].
pub fn set(on: bool) -> Result<()> {
    if in_flatpak() {
        #[cfg(unix)]
        ask_portal(on);
        return Ok(());
    }
    let path = entry_path().ok_or_else(|| AnimaError::other("no configuration directory"))?;
    if on {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        crate::util::atomic_write_bytes(&path, entry(&exec_line()?).as_bytes())?;
        tracing::info!("Start at login: on");
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => tracing::info!("Start at login: off"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// The portal's last answer, taken: `Some(on)` once it has said whether
/// the app now starts with the session.
pub fn take_portal_answer() -> Option<bool> {
    match PORTAL_ANSWER.swap(NO_ANSWER, Ordering::AcqRel) {
        ANSWER_ON => Some(true),
        ANSWER_OFF => Some(false),
        _ => None,
    }
}

const NO_ANSWER: u8 = 0;
const ANSWER_ON: u8 = 1;
const ANSWER_OFF: u8 = 2;
static PORTAL_ANSWER: AtomicU8 = AtomicU8::new(NO_ANSWER);

#[cfg(unix)]
fn ask_portal(on: bool) {
    let spawned = std::thread::Builder::new()
        .name("anima-autostart".into())
        .spawn(move || {
            let answer = async_io::block_on(request_background(on));
            let granted = match answer {
                Ok(granted) => {
                    tracing::info!(
                        "Start at login (portal): {}",
                        if granted { "on" } else { "off" }
                    );
                    granted
                }
                Err(e) => {
                    // Refused, dismissed or no portal: the app does not
                    // start with the session, whatever was asked.
                    tracing::warn!("Start at login: the Background portal said no: {e}");
                    false
                }
            };
            PORTAL_ANSWER.store(
                if granted { ANSWER_ON } else { ANSWER_OFF },
                Ordering::Release,
            );
        });
    if let Err(e) = spawned {
        tracing::warn!("Autostart thread failed to start: {e}");
    }
}

/// `org.freedesktop.portal.Background.RequestBackground`: whether the
/// app now starts with the session.
#[cfg(unix)]
async fn request_background(on: bool) -> std::result::Result<bool, crate::portal::PortalError> {
    use crate::portal::{portal_request, sanitize_sender};
    use std::collections::HashMap;
    use zbus::zvariant::Value;

    let conn = zbus::Connection::session()
        .await
        .map_err(|e| format!("session bus: {e}"))?;
    let sender = sanitize_sender(
        conn.unique_name()
            .ok_or_else(|| "connection has no unique name".to_string())?
            .as_str(),
    );
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Background",
    )
    .await
    .map_err(|e| format!("Background proxy: {e}"))?;
    let reason = crate::i18n::t("appearance-autostart-reason");
    // No timeout: the desktop may ask the user.
    let results = portal_request(&conn, &sender, &proxy, "RequestBackground", None, |token| {
        let mut options: HashMap<&str, Value<'_>> = HashMap::new();
        options.insert("handle_token", Value::from(token));
        options.insert("reason", Value::from(reason));
        options.insert("autostart", Value::from(on));
        // No parent window: the overlay has none the portal understands.
        ("", options)
    })
    .await?;
    Ok(matches!(
        results.get("autostart").map(|v| &**v),
        Some(Value::Bool(true))
    ))
}

/// The command the entry runs: this program — the AppImage, not the
/// mount it runs from, which is gone once it exits — with the native
/// Wayland opt-in carried over when this run has it.
fn exec_line() -> Result<String> {
    let program = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .ok_or_else(|| AnimaError::other("cannot tell where this program is"))?;
    let mut line = String::new();
    if std::env::var("ANIMA_USE_WAYLAND_NATIVE").as_deref() == Ok("1") {
        line.push_str("env ANIMA_USE_WAYLAND_NATIVE=1 ");
    }
    line.push_str(&exec_arg(&program.to_string_lossy()));
    Ok(line)
}

/// One argument of an `Exec` line, per the Desktop Entry spec: quoted if
/// it has a reserved character, with `"` `` ` `` `$` `\` escaped inside;
/// then the value's own escaping doubles every backslash, and a `%` —
/// a field code otherwise — is written `%%`.
fn exec_arg(arg: &str) -> String {
    const RESERVED: &str = " \t\n\"'\\><~|&;$*?#()`";
    let quoted = if arg.chars().any(|c| RESERVED.contains(c)) {
        let mut s = String::from('"');
        for c in arg.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                s.push('\\');
            }
            s.push(c);
        }
        s.push('"');
        s
    } else {
        arg.to_string()
    };
    quoted.replace('\\', "\\\\").replace('%', "%%")
}

fn entry(exec: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=animaEngine\n\
         Comment=Animated characters on your desktop\n\
         Exec={exec}\n\
         Icon=anima-engine\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n\
         # Written by animaEngine's \"Start at login\" (Appearance); switch\n\
         # it off there, or delete this file.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_path_is_left_as_it_is() {
        assert_eq!(exec_arg("/usr/bin/anima-engine"), "/usr/bin/anima-engine");
    }

    #[test]
    fn a_path_with_reserved_characters_is_quoted_per_the_spec() {
        assert_eq!(
            exec_arg("/home/a b/anima"),
            "\"/home/a b/anima\"",
            "a space quotes"
        );
        // `$` escaped by the quoting rule, then that backslash doubled —
        // the spec's own example is \\$.
        assert_eq!(exec_arg("/x/$y"), "\"/x/\\\\$y\"");
        // A literal backslash takes four.
        assert_eq!(exec_arg("/x/a\\b"), "\"/x/a\\\\\\\\b\"");
        // A percent sign is not a field code.
        assert_eq!(exec_arg("/x/100%"), "/x/100%%");
    }

    #[test]
    fn the_entry_runs_the_line_given() {
        let e = entry("\"/opt/my apps/anima\"");
        assert!(e.starts_with("[Desktop Entry]\n"));
        assert!(e.contains("\nExec=\"/opt/my apps/anima\"\n"));
        assert!(e.contains("\nType=Application\n"));
    }

    #[test]
    fn no_answer_until_the_portal_gives_one() {
        assert_eq!(take_portal_answer(), None);
        PORTAL_ANSWER.store(ANSWER_ON, Ordering::Release);
        assert_eq!(take_portal_answer(), Some(true));
        assert_eq!(take_portal_answer(), None, "taken once");
    }
}
