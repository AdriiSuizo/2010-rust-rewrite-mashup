//! Pack-backed pinned 26.3 creeper: `CreeperModel.createBodyLayer` (a head
//! and body on four legs), its `setupAnim` (head turn and pitch, the legs
//! swinging in diagonal pairs with the walk), and `CreeperRenderer`'s
//! swell (`scale`: wider and slightly taller, with a wobble) and white fuse
//! flash (`getWhiteOverlayProgress`). Vanilla mixes the white overlay into
//! the texture; here the vertex colour brightens it, as for primed TNT.
//! The charged creeper's energy layer is not drawn yet.
use crate::{
    cow_render::cube_scaled,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
    walk_animation::WalkAnimations,
};
use glam::{Quat, Vec3};
use minecraftoss_entities::world::CreeperEntity;

type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3]);

const HEAD: Part = ([-4., -8., -4.], [4., 0., 4.], [0., 0.], [0., 6., 0.]);
const BODY: Part = ([-4., 0., -2.], [4., 12., 2.], [16., 16.], [0., 6., 0.]);
/// The legs' pivots, each with the phase of its swing: the model's
/// "right_hind_leg" and "left_front_leg" pair up, as do the other two.
const LEGS: [([f32; 3], f32); 4] = [
    ([-2., 18., 4.], std::f32::consts::PI),
    ([2., 18., 4.], 0.0),
    ([-2., 18., -4.], 0.0),
    ([2., 18., -4.], std::f32::consts::PI),
];

/// `CreeperRenderer.scale`: the swell widens the creeper by up to 40%
/// and heightens it by up to 10%, wobbling as it goes.
pub fn swell_scale(swelling: f32) -> Vec3 {
    let wobble = 1.0 + (swelling * 100.0).sin() * swelling * 0.01;
    let g = swelling.clamp(0.0, 1.0);
    let g = g * g;
    let g = g * g;
    let s = (1.0 + g * 0.4) * wobble;
    let hs = (1.0 + g * 0.1) / wobble;
    Vec3::new(s, hs, s)
}

/// `CreeperRenderer.getWhiteOverlayProgress`: white on alternate tenths of
/// the swell, from half strength.
pub fn white_overlay(swelling: f32) -> f32 {
    if (swelling * 10.0) as i32 % 2 == 0 {
        0.0
    } else {
        swelling.clamp(0.5, 1.0)
    }
}

pub fn append_creepers<'a>(
    mesh: &mut ChunkMesh,
    creepers: impl IntoIterator<Item = &'a CreeperEntity>,
    walks: &WalkAnimations,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let texture = ResourceId::parse("minecraft:entity/creeper/creeper").unwrap();
    let region = atlas.entity_region(&texture);
    let partial = partial.clamp(0.0, 1.0);
    for entity in creepers {
        let creeper = &entity.creeper;
        if creeper.health <= 0.0 || creeper.exploded {
            continue;
        }
        let feet = entity.previous_position.lerp(creeper.body.position, f64::from(partial));
        let eye_height = creeper.body.height * 0.85;
        let sample = (feet.x.floor() as i32, (feet.y + f64::from(eye_height)).floor() as i32, feet.z.floor() as i32);
        let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
        let body_yaw = entity.ai.body_rotation.body_yaw;
        let rotation = Quat::from_rotation_y(std::f32::consts::PI - body_yaw.to_radians());
        let look = &entity.ai.state.look_control;
        let head = Quat::from_euler(glam::EulerRot::ZYX, 0.0, (look.head_yaw - body_yaw).to_radians(), look.pitch.to_radians());
        // `Creeper.getSwelling`: the fuse between ticks over two short of
        // its length.
        let swell = creeper.old_swell as f32 + partial * (creeper.swell - creeper.old_swell) as f32;
        let swelling = swell / (creeper.max_swell - 2) as f32;
        let scale = swell_scale(swelling);
        // The overlay's texture column (`OverlayTexture.u`) sets how much
        // white shows: up to 75%.
        let white = (white_overlay(swelling) * 15.0) as i32 as f32 / 15.0 * 0.75;
        let tint = [1.0 + 4.0 * white; 3];
        let walk = walks.get(entity.id);
        let (swing, speed) = (walk.position(partial), walk.speed(partial));
        let mut parts: Vec<(Part, Quat)> = vec![(HEAD, head), (BODY, Quat::IDENTITY)];
        for (pivot, phase) in LEGS {
            let angle = (swing * 0.6662 + phase).cos() * 1.4 * speed;
            parts.push((([-2., 0., -2.], [2., 6., 2.], [0., 16.], pivot), Quat::from_rotation_x(angle)));
        }
        for ((from, to, uv, pivot), pose) in parts {
            cube_scaled(mesh, feet, rotation, scale, region, sky, block, from, to, uv, pivot, pose, tint, [64., 32.], None, false);
        }
    }
}
