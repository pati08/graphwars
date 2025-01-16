use bevy::prelude::Color;
use std::time::Duration;

/// Radius of soldiers in pixels
pub const SOLDIER_RADIUS: f32 = 12.;

/// Color with which to outline the active soldier
pub const ACTIVE_SOLDIER_OUTLINE_COLOR: Color = Color::srgb(0., 1., 0.);

/// Steps in x to take when graphing
pub const GRAPH_RES: f32 = 0.01;

/// Speed to graph at (units/sec)
pub const GRAPHING_SPEED: f32 = 20.;

/// The function to use before the player customises it
pub const DEFAULT_FUNCTION: &str = "x";

/// The slope to require over a step to consider a graph discontinuous
pub const DISCONTINUITY_THRESHOLD: f32 = 15.;

/// How long to wait after graphing to start the next turn
pub const AFTER_GRAPH_PAUSE: Duration = Duration::from_secs(1);

/// Size of explosion sprite in pixels
pub const EXPLOSION_SPRITE_SIZE: f32 = 35.;

/// Original size of explosion sprite image
pub const EXPLOSION_IMAGE_SIZE: f32 = 128.;

// Z indices of different elements
pub const GRID_BACKGROUND_Z: f32 = -10.;
pub const SOLDIER_Z: f32 = 10.;
pub const PLAYER_NAME_Z: f32 = 15.;
pub const SOLDIER_NAME_Z: f32 = 15.;
pub const EXPLOSION_Z: f32 = 20.;
