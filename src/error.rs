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

    #[error("X11 connection error: {0}")]
    X11Connect(#[from] x11rb::errors::ConnectError),

    #[error("X11 protocol error: {0}")]
    X11Reply(#[from] x11rb::errors::ReplyError),

    #[error("X11 connection lost: {0}")]
    X11Connection(#[from] x11rb::errors::ConnectionError),

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
}

impl AnimaError {
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
