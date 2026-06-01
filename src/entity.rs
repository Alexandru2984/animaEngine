use crate::animation::Animation;
use crate::config::CharacterConfig;
use crate::physics::PhysicsState;

/// A single animated entity on screen.
/// Represents one character/asset with its position, appearance, and animation state.
#[derive(Debug)]
pub struct Entity {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Position on screen (pixels from top-left)
    pub x: f32,
    pub y: f32,
    /// Scale factor (1.0 = original size)
    pub scale: f32,
    /// Opacity (0.0 = invisible, 1.0 = fully opaque)
    pub opacity: f32,
    /// Z-ordering (higher = on top)
    pub z_index: i32,
    /// Whether this entity is visible
    pub visible: bool,
    /// Animation state
    pub animation: Animation,
    /// Whether the GPU texture needs updating (frame changed)
    pub texture_dirty: bool,
    /// The original asset path (for config saving)
    pub asset_path: String,
    /// Asset type (for config saving)
    pub asset_type: crate::config::AssetType,
    /// Spritesheet columns (for config saving, only used for Spritesheet type)
    pub spritesheet_columns: Option<u32>,
    /// Spritesheet rows (for config saving, only used for Spritesheet type)
    pub spritesheet_rows: Option<u32>,
    /// Physics state (gravity, velocity, grounded)
    pub physics: PhysicsState,
}

impl Entity {
    /// Create an entity from config + loaded animation frames
    pub fn from_config(config: &CharacterConfig, animation: Animation) -> Self {
        Self {
            id: config.id.clone(),
            name: config.name.clone(),
            x: config.x,
            y: config.y,
            scale: config.scale,
            opacity: config.opacity,
            z_index: config.z_index,
            visible: config.visible,
            animation,
            texture_dirty: true, // Needs initial texture upload
            asset_path: config.asset_path.clone(),
            asset_type: config.asset_type.clone(),
            spritesheet_columns: config.spritesheet_columns,
            spritesheet_rows: config.spritesheet_rows,
            physics: PhysicsState::from_enabled(config.physics_enabled),
        }
    }

    /// Tick the entity: animation + physics
    /// `dt` = delta time in seconds, `screen_height` = screen height for floor collision
    pub fn tick(&mut self, dt: f32, screen_height: f32) -> bool {
        // Update physics (gravity, bounce)
        let sprite_h = self.scaled_height();
        self.y = self.physics.tick(self.y, sprite_h, screen_height, dt);

        // Update animation
        if self.animation.tick() {
            self.texture_dirty = true;
            return true;
        }
        false
    }

    /// Get the current frame dimensions (scaled)
    pub fn scaled_width(&self) -> f32 {
        self.animation
            .current_frame_data()
            .map(|f| f.width as f32 * self.scale)
            .unwrap_or(64.0)
    }

    pub fn scaled_height(&self) -> f32 {
        self.animation
            .current_frame_data()
            .map(|f| f.height as f32 * self.scale)
            .unwrap_or(64.0)
    }

    /// Check if a point (in screen coords) hits this entity
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        if !self.visible {
            return false;
        }
        let w = self.scaled_width();
        let h = self.scaled_height();
        px >= self.x && px <= self.x + w && py >= self.y && py <= self.y + h
    }

    /// Convert this entity's state back to a CharacterConfig for saving
    pub fn to_config(&self) -> CharacterConfig {
        CharacterConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            asset_type: self.asset_type.clone(),
            asset_path: self.asset_path.clone(),
            x: self.x,
            y: self.y,
            scale: self.scale,
            opacity: self.opacity,
            fps: self.animation.fps,
            visible: self.visible,
            playing: self.animation.playing,
            z_index: self.z_index,
            physics_enabled: self.physics.enabled,
            spritesheet_columns: self.spritesheet_columns,
            spritesheet_rows: self.spritesheet_rows,
        }
    }
}
