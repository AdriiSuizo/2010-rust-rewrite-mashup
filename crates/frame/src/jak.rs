use std::sync::Arc;

use bevy::prelude::*;

/// Skinning matrices for Jak's own model, his board, his gun and Light
/// Jak's wings, one per
/// joint in the body's frame; empty when that part is not shown.
#[derive(Clone, Debug, Default)]
pub struct JakSkins {
    pub body: Vec<Mat4>,
    pub board: Vec<Mat4>,
    pub gun: Vec<Mat4>,
    /// Light Jak's wings.
    pub wings: Vec<Mat4>,
}

/// Jak Mode as the presentation sees it. The gameplay is `jak_mode`'s; this
/// is what the body, the camera and the HUD draw from.
#[derive(Resource, Default)]
pub struct JakMode {
    pub active: bool,
    pub toggle_requested: bool,
    pub input_blocked: bool,
    pub client: u32,
    /// Jak's body, feet at the origin, in map space.
    pub root: Mat4,
    /// The placeholder board under his feet while he rides it, in the
    /// body's frame.
    pub board: Option<Mat4>,
    /// Jak's own model, posed, when the player's exported model is present.
    pub skins: Option<Arc<JakSkins>>,
    pub camera: Option<(Transform, f32)>,
    /// Shots in flight: head and the end of the beam behind it, map space.
    pub shots: Vec<(Vec3, Vec3)>,
    pub state: String,
    pub speed: f32,
    pub ammo: Option<f32>,
    /// Light Jak, and his light eco.
    pub light: bool,
    pub light_eco: f32,
    pub status: String,
    /// The state machine's readout, filled while `show_debug` is on.
    pub show_debug: bool,
    pub debug: String,
}
