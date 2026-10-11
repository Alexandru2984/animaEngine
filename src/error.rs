use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AnimaError>;

#[derive(Debug, Error)]
pub enum AnimaError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image decode error: {0}")]
    ImageDecode(#[from] image::ImageError),

    #[error("config parse error: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("config serialize error: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),

    #[error("wgpu surface error: {0}")]
    WgpuSurface(#[from] wgpu::CreateSurfaceError),

    #[error("wgpu device request error: {0}")]
    WgpuDevice(#[from] wgpu::RequestDeviceError),

    #[error("no suitable GPU adapter found")]
    NoAdapter,

    // x11rb is a unix-only dependency (see Cargo.toml), so the variants
    // that wrap its error types only exist there. Off-unix the X11 backend
    // isn't compiled either, so nothing can construct them.
    #[cfg(unix)]
    #[error("X11 connection error: {0}")]
    X11Connect(#[from] x11rb::errors::ConnectError),

    #[cfg(unix)]
    #[error("X11 protocol error: {0}")]
    X11Reply(#[from] x11rb::errors::ReplyError),

    #[cfg(unix)]
    #[error("X11 connection lost: {0}")]
    X11Connection(#[from] x11rb::errors::ConnectionError),

    #[cfg(unix)]
    #[error("X11 ID exhausted: {0}")]
    X11Id(#[from] x11rb::errors::ReplyOrIdError),

    #[error("asset not found: {0}")]
    AssetNotFound(PathBuf),

    #[error("asset path is not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("no frames found in {0}")]
    EmptyAsset(PathBuf),

    #[error("image too large: {width}×{height} (max {max}×{max})")]
    ImageTooLarge { width: u32, height: u32, max: u32 },

    #[error("invalid spritesheet: {0}")]
    InvalidSpritesheet(String),

    #[error("video decode error: {0}")]
    VideoDecode(String),

    #[error("frame buffer corrupted: expected {expected} bytes, got {got}")]
    FrameBufferCorrupt { expected: usize, got: usize },

    #[error("{0}")]
    Other(String),

    /// An error this app words itself, for the user's language
    /// ([`AnimaError::said`]); in English in the log.
    #[error("{0}")]
    Said(crate::i18n::Msg),

    /// `error`, met on the way through `name` — a character, a file in
    /// an archive.
    #[error("{name}: {error}")]
    For {
        name: String,
        error: Box<AnimaError>,
    },
}

impl AnimaError {
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    /// The message `id` from the `.ftl` files (`crate::i18n::Msg`).
    pub fn said_as(msg: crate::i18n::Msg) -> Self {
        Self::Said(msg)
    }

    /// This error, met on the way through `name`.
    pub fn for_(self, name: impl Into<String>) -> Self {
        Self::For {
            name: name.into(),
            error: Box::new(self),
        }
    }

    /// What to tell the user, in their language: this app's own words
    /// translated; a system's or a library's — an I/O error, a decoder's
    /// — as they come, in English.
    pub fn said(&self) -> String {
        use crate::i18n::Msg;
        let path = |p: &PathBuf| crate::drop_validate::redact_path(p);
        match self {
            Self::Said(msg) => msg.said(),
            Self::For { name, error } => format!("{name}: {}", error.said()),
            Self::AssetNotFound(p) => Msg::new("error-asset-not-found")
                .arg("path", path(p))
                .said(),
            Self::NotADirectory(p) => Msg::new("error-not-a-folder").arg("path", path(p)).said(),
            Self::EmptyAsset(p) => Msg::new("error-no-frames").arg("path", path(p)).said(),
            Self::ImageTooLarge { width, height, max } => Msg::new("error-image-too-large")
                .arg("width", width)
                .arg("height", height)
                .arg("max", max)
                .said(),
            other => other.to_string(),
        }
    }
}
