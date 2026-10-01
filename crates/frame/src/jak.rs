use bevy::prelude::*;

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
    /// The board under his feet while he rides it, in map space.
    pub board: Option<Mat4>,
    pub camera: Option<(Transform, f32)>,
    /// Shots in flight: head and the end of the beam behind it, map space.
    pub shots: Vec<(Vec3, Vec3)>,
    pub state: String,
    pub speed: f32,
    pub ammo: Option<f32>,
    pub status: String,
}
