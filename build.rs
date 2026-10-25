//! Windows resources: the icon Explorer, the taskbar, the installer and
//! the tray show (`win_tray`), and the version details a file's
//! Properties list. Nothing on any other target.
//!
//! The resource script is written here and compiled by the toolchain's
//! own resource compiler — mingw's `windres` for the GNU target, `rc.exe`
//! for MSVC — so no crate is needed. Without one the build goes on, the
//! `.exe` with no icon, and says so.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=data/anima-engine.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let icon = Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("anima-engine.ico");
    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let numbers: Vec<&str> = version
        .split(['.', '-', '+'])
        .take(3)
        .chain(["0", "0", "0"])
        .take(3)
        .collect();
    let numeric = format!("{},{},{},0", numbers[0], numbers[1], numbers[2]);
    // ASCII only: a resource script is read in the build machine's code
    // page.
    let script = format!(
        r#"1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "animaEngine"
      VALUE "FileDescription", "animaEngine - animated characters on the desktop"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "anima-engine"
      VALUE "LegalCopyright", "Copyright (c) animaEngine contributors. MIT licence."
      VALUE "OriginalFilename", "anima-engine.exe"
      VALUE "ProductName", "animaEngine"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        icon = icon.display().to_string().replace('\\', "/"),
    );
    let rc = out.join("anima.rc");
    std::fs::write(&rc, script).expect("write the resource script");

    let compiled = if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let res = out.join("anima.res");
        let ok = Command::new("rc.exe")
            .arg("/nologo")
            .arg("/fo")
            .arg(&res)
            .arg(&rc)
            .status()
            .is_ok_and(|s| s.success());
        ok.then_some(res)
    } else {
        let obj = out.join("anima-res.o");
        let target = std::env::var("TARGET").unwrap_or_default();
        let prefixed = match target.as_str() {
            "x86_64-pc-windows-gnu" => Some("x86_64-w64-mingw32-windres"),
            "i686-pc-windows-gnu" => Some("i686-w64-mingw32-windres"),
            _ => None,
        };
        prefixed.into_iter().chain(["windres"]).find_map(|windres| {
            Command::new(windres)
                .args(["--input-format=rc", "--output-format=coff", "-i"])
                .arg(&rc)
                .arg("-o")
                .arg(&obj)
                .status()
                .is_ok_and(|s| s.success())
                .then(|| obj.clone())
        })
    };
    match compiled {
        Some(resource) => println!("cargo:rustc-link-arg-bins={}", resource.display()),
        None => {
            println!("cargo:warning=no resource compiler: the .exe has no icon or version details")
        }
    }
}
