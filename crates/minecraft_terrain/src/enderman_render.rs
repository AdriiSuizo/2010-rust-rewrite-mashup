//! Pack-backed pinned 26.3 enderman: `EndermanModel.createBodyLayer` (the
//! humanoid mesh raised by 14, with a hat inside the head and arms and legs
//! 30 long), its `setupAnim` over `HumanoidModel`'s (the head's turn and
//! pitch, the limbs' walk swing and the arms' bob, halved and held within
//! 0.4; arms out for a carried block; a creepy enderman's head lifted off
//! its jaw), `EnderEyesLayer` (drawn at full brightness where vanilla adds
//! it unlit), `CarriedBlockLayer`'s pose for the block in its hands, and
//! `EndermanRenderer.getRenderOffset`'s shaking while creepy.
use crate::{
    cow_render::cube_tinted_pose_mirror,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
    walk_animation::WalkAnimations,
};
use glam::{DVec3, EulerRot, Mat4, Quat, Vec3};
use minecraftoss_entities::world::EndermanEntity;
use std::f32::consts::PI;

/// A cuboid: corners, texture offset, pivot, mirrored, the size its UVs
/// come from when inflated, and its pose (0 body, 1 head, 2 right arm,
/// 3 left arm, 4 right leg, 5 left leg, 6 the hat inside the head).
type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3], bool, Option<[f32; 3]>, u8);

const PARTS: [Part; 7] = [
    ([-4., -8., -4.], [4., 0., 4.], [0., 0.], [0., -13., 0.], false, None, 1),
    // `CubeDeformation(-0.5)`.
    ([-3.5, -7.5, -3.5], [3.5, -0.5, 3.5], [0., 16.], [0., -13., 0.], false, Some([8., 8., 8.]), 6),
    ([-4., 0., -2.], [4., 12., 2.], [32., 16.], [0., -14., 0.], false, None, 0),
    ([-1., -2., -1.], [1., 28., 1.], [56., 0.], [-5., -12., 0.], false, None, 2),
    ([-1., -2., -1.], [1., 28., 1.], [56., 0.], [5., -12., 0.], true, None, 3),
    ([-1., 0., -1.], [1., 30., 1.], [56., 0.], [-2., -5., 0.], false, None, 4),
    ([-1., 0., -1.], [1., 30., 1.], [56., 0.], [2., -5., 0.], true, None, 5),
];

/// The limbs' pitch and roll: `HumanoidModel.setupAnim`'s swing and bob
/// (`AnimationUtils.bobModelPart`), then `EndermanModel`'s halving and
/// ±0.4 clamp, or the arms held out for a block.
pub fn limb_angles(walk_position: f32, walk_speed: f32, age: f32, carrying: bool) -> [(f32, f32); 4] {
    let p = walk_position * 0.6662;
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;
    let bob_x = (age * 0.067).sin() * 0.05;
    let right_arm = (((p + PI).cos() * 2.0 * walk_speed * 0.5) + bob_x, bob_z);
    let left_arm = ((p.cos() * 2.0 * walk_speed * 0.5) - bob_x, -bob_z);
    let right_leg = (p.cos() * 1.4 * walk_speed, 0.0);
    let left_leg = ((p + PI).cos() * 1.4 * walk_speed, 0.0);
    let limit = |(x, z): (f32, f32)| ((x * 0.5).clamp(-0.4, 0.4), z);
    let (mut right_arm, mut left_arm) = (limit(right_arm), limit(left_arm));
    if carrying {
        right_arm = (-0.5, 0.05);
        left_arm = (-0.5, -0.05);
    }
    [right_arm, left_arm, limit(right_leg), limit(left_leg)]
}

/// `CarriedBlockLayer`'s pose for a block model in `[0, 1]³`, in the
/// model's block units (y down from the model's origin, 1.5 above the
/// feet).
fn carried_block_pose() -> Mat4 {
    Mat4::from_translation(Vec3::new(0.0, 0.6875, -0.75))
        * Mat4::from_rotation_x(20f32.to_radians())
        * Mat4::from_rotation_y(45f32.to_radians())
        * Mat4::from_translation(Vec3::new(0.25, 0.1875, 0.25))
        * Mat4::from_scale(Vec3::new(-0.5, -0.5, 0.5))
        * Mat4::from_rotation_y(90f32.to_radians())
}

/// Appends the endermen and returns each carried block's model pose (world
/// space, for a block model in `[0, 1]³`), where it is lit, and its block.
pub fn append_endermen<'a>(
    mesh: &mut ChunkMesh,
    endermen: impl IntoIterator<Item = &'a EndermanEntity>,
    walks: &WalkAnimations,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
    frame: u64,
) -> Vec<(Mat4, Vec3, String)> {
    let skin = atlas.entity_region(&ResourceId::parse("minecraft:entity/enderman/enderman").unwrap());
    let eyes_id = ResourceId::parse("minecraft:entity/enderman/enderman_eyes").unwrap();
    let eyes = atlas.contains(&eyes_id).then(|| atlas.entity_region(&eyes_id));
    let partial = partial.clamp(0.0, 1.0);
    let mut carried = Vec::new();
    for entity in endermen {
        if entity.enderman.health <= 0.0 {
            continue;
        }
        let mut feet = entity.previous_position.lerp(entity.enderman.body.position, f64::from(partial));
        let creepy = entity.creepy();
        if creepy {
            // `getRenderOffset`: a gaussian shake of 0.02 a side.
            let (dx, dz) = shake(entity.id, frame);
            feet += DVec3::new(dx * 0.02, 0.0, dz * 0.02);
        }
        let eye = feet + DVec3::new(0.0, f64::from(minecraftoss_entities::enderman::EYE_HEIGHT), 0.0);
        let sample = (eye.x.floor() as i32, eye.y.floor() as i32, eye.z.floor() as i32);
        let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
        let body_yaw = entity.ai.body_rotation.body_yaw;
        let rotation = Quat::from_rotation_y(PI - body_yaw.to_radians());
        let look = &entity.ai.state.look_control;
        let head = Quat::from_euler(EulerRot::ZYX, 0.0, (look.head_yaw - body_yaw).to_radians(), look.pitch.to_radians());
        let walk = walks.get(entity.id);
        let age = entity.tick_count as f32 + partial;
        let limbs = limb_angles(walk.position(partial), walk.speed(partial), age, entity.carried().is_some());
        // Creepy: the head rises five pixels, the hat (its jaw) stays.
        let head_pivot = Vec3::new(0.0, if creepy { -18.0 } else { -13.0 }, 0.0);
        let hat_pivot = head_pivot + head * Vec3::new(0.0, if creepy { 5.0 } else { 0.0 }, 0.0);
        let pose_of = |pose: u8| -> (Quat, Option<Vec3>) {
            match pose {
                1 => (head, Some(head_pivot)),
                6 => (head, Some(hat_pivot)),
                2..=5 => {
                    let (x, z) = limbs[usize::from(pose - 2)];
                    (Quat::from_euler(EulerRot::ZYX, z, 0.0, x), None)
                }
                _ => (Quat::IDENTITY, None),
            }
        };
        let mut draw = |region: [f32; 4], sky: f32, block: f32| {
            for (from, to, uv, pivot, mirror, uv_size, pose) in PARTS {
                let (part, moved) = pose_of(pose);
                let pivot = moved.map_or(pivot, |p| p.to_array());
                cube_tinted_pose_mirror(mesh, feet, rotation, 1.0, region, sky, block, from, to, uv, pivot, part, [1.0; 3], [64., 32.], uv_size, mirror);
            }
        };
        draw(skin, sky, block);
        if let Some(eyes) = eyes {
            draw(eyes, 15.0, 15.0);
        }
        if let Some(held) = entity.carried() {
            // `LivingEntityRenderer`'s model space: `scale(-1, -1, 1)` and
            // the 1.501 lift, turned with the body.
            let flip = Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0)) * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));
            let world = Mat4::from_translation(feet.as_vec3()) * Mat4::from_quat(rotation) * flip * carried_block_pose();
            carried.push((world, eye.as_vec3(), held.id.clone()));
        }
    }
    carried
}

/// Two standard-normal-ish offsets for a creepy enderman's shake, fresh
/// each frame (vanilla draws them from the renderer's own random).
fn shake(id: u64, frame: u64) -> (f64, f64) {
    let mut state = id.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ frame.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let mut uniform = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64
    };
    // Box–Muller.
    let (u, v) = (uniform().max(1e-12), uniform());
    let r = (-2.0 * u.ln()).sqrt();
    (r * (std::f64::consts::TAU * v).cos(), r * (std::f64::consts::TAU * v).sin())
}
