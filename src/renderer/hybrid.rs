//! Leaving a laptop's NVIDIA GPU asleep (1.5).
//!
//! A laptop with an integrated AMD or Intel GPU beside an NVIDIA one puts
//! the NVIDIA GPU to sleep while nothing uses it. Asked which GPUs there
//! are, the Vulkan loader loads every driver installed, NVIDIA's among
//! them, and that wakes it: on an RTX 5060 laptop the answer took 2.9 s
//! instead of 0.09, and closing the driver at quit woke it again, the
//! process gone 2.8 s after its config was saved — the GPU awake some
//! 20 s each time. The renderer asks for the low-power GPU and draws on
//! the integrated one regardless, so on such a laptop NVIDIA's Vulkan
//! driver is not loaded at all: `VK_LOADER_DRIVERS_DISABLE=*nvidia*`, set
//! before any thread exists — the environment is only safe to change
//! while nothing else can be reading it.
//!
//! Not when the user chose: the loader's own variables, or a PRIME
//! offload (`prime-run`, a desktop's "Launch using Discrete Graphics
//! Card", `DRI_PRIME`), which asks for the discrete GPU. Not when no other
//! GPU is AMD's or Intel's, where NVIDIA's may be the only one. And not
//! when the NVIDIA GPU is awake as the app starts: then something uses it
//! — a session drawn on it (`prime-select nvidia`), where the integrated
//! GPU's driver may not reach the windows and NVIDIA's is the one to draw
//! with — and loading its driver wakes nothing. Should Vulkan find
//! nothing but software where the driver was left out, the renderer tries
//! GL (`choose_gpu`), whose loader still reaches every vendor.

use std::path::Path;

const NVIDIA: u16 = 0x10de;
const AMD: u16 = 0x1002;
const INTEL: u16 = 0x8086;

/// Set, each means the user chose a GPU or the loader's drivers.
const CHOSEN: &[&str] = &[
    "VK_LOADER_DRIVERS_SELECT",
    "VK_LOADER_DRIVERS_DISABLE",
    "VK_DRIVER_FILES",
    "VK_ICD_FILENAMES",
    "VK_ADD_DRIVER_FILES",
    "__NV_PRIME_RENDER_OFFLOAD",
    "__VK_LAYER_NV_optimus",
    "DRI_PRIME",
    "MESA_VK_DEVICE_SELECT",
];

/// What [`spare_discrete_gpu`] did, for the log once there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// NVIDIA's Vulkan driver is not loaded.
    Spared,
    /// The user chose, through this variable.
    UserChose(&'static str),
    /// Not an NVIDIA GPU beside an AMD or Intel one.
    NotHybrid,
    /// The NVIDIA GPU is awake: something uses it.
    Awake,
}

impl Outcome {
    pub fn log(self) {
        match self {
            Self::Spared => tracing::info!(
                "NVIDIA GPU beside an integrated one: its Vulkan driver is not \
                 loaded, so it can sleep (prime-run, or __NV_PRIME_RENDER_OFFLOAD=1, \
                 to draw on it)"
            ),
            Self::UserChose(name) => {
                tracing::info!("{name} is set: every Vulkan driver is loaded, as asked")
            }
            Self::Awake => tracing::info!(
                "NVIDIA GPU beside an integrated one, awake: its Vulkan driver \
                 is loaded, as it is in use"
            ),
            Self::NotHybrid => {}
        }
    }
}

/// Leave NVIDIA's Vulkan driver unloaded on a laptop that has another GPU
/// to draw on, unless the user chose otherwise. Call first thing in
/// `main`, before any thread is made.
pub fn spare_discrete_gpu() -> Outcome {
    let outcome = decide(
        |name| std::env::var_os(name).is_some(),
        &gpus(Path::new("/sys/class/drm")),
    );
    if outcome == Outcome::Spared {
        // Sound: `main` calls this before it makes a thread, so nothing
        // reads the environment meanwhile.
        std::env::set_var("VK_LOADER_DRIVERS_DISABLE", "*nvidia*");
    }
    outcome
}

/// A GPU the kernel has a DRM card for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Gpu {
    /// Its PCI vendor.
    vendor: u16,
    /// Put to sleep by runtime power management: nothing uses it.
    asleep: bool,
}

fn decide(is_set: impl Fn(&str) -> bool, gpus: &[Gpu]) -> Outcome {
    if let Some(name) = CHOSEN.iter().find(|name| is_set(name)) {
        return Outcome::UserChose(name);
    }
    let mut nvidia = gpus.iter().filter(|gpu| gpu.vendor == NVIDIA).peekable();
    let other = gpus
        .iter()
        .any(|gpu| gpu.vendor == AMD || gpu.vendor == INTEL);
    if nvidia.peek().is_none() || !other {
        Outcome::NotHybrid
    } else if nvidia.all(|gpu| gpu.asleep) {
        Outcome::Spared
    } else {
        Outcome::Awake
    }
}

/// Each GPU the kernel has a DRM card for: `cardN`, not its connectors
/// (`cardN-eDP-1`) nor render nodes. A card with no PCI device behind it
/// (simpledrm's, a VM's virtual one) has no vendor file and is left out.
fn gpus(drm: &Path) -> Vec<Gpu> {
    let Ok(entries) = std::fs::read_dir(drm) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.strip_prefix("card")
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .filter_map(|entry| {
            let device = entry.path().join("device");
            let vendor = std::fs::read_to_string(device.join("vendor")).ok()?;
            let vendor = u16::from_str_radix(vendor.trim().trim_start_matches("0x"), 16).ok()?;
            let asleep = std::fs::read_to_string(device.join("power/runtime_status"))
                .is_ok_and(|status| status.trim() == "suspended");
            Some(Gpu { vendor, asleep })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nothing_set(_: &str) -> bool {
        false
    }

    const fn awake(vendor: u16) -> Gpu {
        Gpu {
            vendor,
            asleep: false,
        }
    }

    const fn asleep(vendor: u16) -> Gpu {
        Gpu {
            vendor,
            asleep: true,
        }
    }

    #[test]
    fn a_sleeping_nvidia_gpu_beside_an_integrated_one_is_spared() {
        assert_eq!(
            decide(nothing_set, &[awake(AMD), asleep(NVIDIA)]),
            Outcome::Spared
        );
        assert_eq!(
            decide(nothing_set, &[asleep(NVIDIA), awake(INTEL)]),
            Outcome::Spared
        );
    }

    #[test]
    fn an_nvidia_gpu_in_use_keeps_its_driver() {
        assert_eq!(
            decide(nothing_set, &[awake(AMD), awake(NVIDIA)]),
            Outcome::Awake
        );
        assert_eq!(
            decide(nothing_set, &[awake(AMD), asleep(NVIDIA), awake(NVIDIA)]),
            Outcome::Awake
        );
    }

    #[test]
    fn nvidia_alone_or_no_nvidia_is_left_as_it_was() {
        assert_eq!(decide(nothing_set, &[asleep(NVIDIA)]), Outcome::NotHybrid);
        assert_eq!(decide(nothing_set, &[awake(AMD)]), Outcome::NotHybrid);
        assert_eq!(decide(nothing_set, &[]), Outcome::NotHybrid);
        // A VM's virtual GPU is not one to draw on in its place.
        assert_eq!(
            decide(nothing_set, &[awake(0x1af4), asleep(NVIDIA)]),
            Outcome::NotHybrid
        );
    }

    #[test]
    fn a_choice_the_user_made_stands() {
        for name in [
            "__NV_PRIME_RENDER_OFFLOAD",
            "DRI_PRIME",
            "VK_LOADER_DRIVERS_DISABLE",
        ] {
            assert_eq!(
                decide(|n| n == name, &[awake(AMD), asleep(NVIDIA)]),
                Outcome::UserChose(name)
            );
        }
    }

    #[test]
    fn gpus_are_read_from_cards_only() {
        let dir = std::env::temp_dir().join(format!("anima_hybrid_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (entry, vendor, status) in [
            ("card0", Some("0x10de\n"), Some("suspended\n")),
            ("card1", Some("0x1002\n"), Some("active\n")),
            ("card1-eDP-1", Some("0x8086\n"), None),
            ("renderD128", Some("0x8086\n"), None),
            ("card2", None, None),
        ] {
            let device = dir.join(entry).join("device");
            std::fs::create_dir_all(device.join("power")).unwrap();
            if let Some(vendor) = vendor {
                std::fs::write(device.join("vendor"), vendor).unwrap();
            }
            if let Some(status) = status {
                std::fs::write(device.join("power/runtime_status"), status).unwrap();
            }
        }
        let mut found = gpus(&dir);
        found.sort_by_key(|gpu| gpu.vendor);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found, vec![awake(AMD), asleep(NVIDIA)]);
        assert!(gpus(Path::new("/nonexistent/drm")).is_empty());
    }
}
