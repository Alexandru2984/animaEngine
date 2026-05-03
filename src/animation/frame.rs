/// Frame data — raw RGBA pixels for a single animation frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// RGBA pixel data
    pub rgba: Vec<u8>,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl Frame {
    pub fn new(rgba: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            rgba,
            width,
            height,
        }
    }
}
