//! Pack-backed pinned 26.3 witch: `WitchModel.createBodyLayer` over
//! `VillagerModel.createBodyModel` (its hat and brim replaced by the
//! witch's crooked four-part hat, a mole on the nose), `WitchModel.setupAnim`
//! (the head's turn and pitch, the legs' walk swing, the nose's slow wobble
//! and, while it holds a potion, the nose raised), from
//! `entity/witch/witch` (64 by 128). Thrown splash potions are drawn as
//! `ThrownItemRenderer` draws them: the item facing the camera at its
//! ground size (`item/splash_potion` over its overlay tinted with the
//! potion's colour).
use crate::{
    cow_render::cube_tinted_pose_mirror,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh, Vertex},
    pack::ResourceId,
    walk_animation::WalkAnimations,
};
use glam::{EulerRot, Quat, Vec2, Vec3};
use minecraftoss_entities::world::{PotionEntity, WitchEntity};
use std::f32::consts::PI;

/// Where a part sits in the model: its rotation and origin (pixels, y
/// down), composed down the part tree.
#[derive(Clone, Copy)]
struct Pose {
    rotation: Quat,
    origin: Vec3,
}

impl Pose {
    const ROOT: Pose = Pose { rotation: Quat::IDENTITY, origin: Vec3::ZERO };

    /// `ModelPart.translateAndRotate` for a child at `offset` turned by
    /// `ZYX(z, y, x)`.
    fn child(self, offset: [f32; 3], x: f32, y: f32, z: f32) -> Pose {
        Pose { rotation: self.rotation * Quat::from_euler(EulerRot::ZYX, z, y, x), origin: self.origin + self.rotation * Vec3::from_array(offset) }
    }
}

/// A cuboid: corners, texture offset, mirrored, the size its UVs come from
/// when inflated, and the part it belongs to.
type Cube = ([f32; 3], [f32; 3], [f32; 2], bool, Option<[f32; 3]>, Part);

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Head,
    Hat,
    Hat2,
    Hat3,
    Hat4,
    Nose,
    Mole,
    Body,
    Arms,
    RightLeg,
    LeftLeg,
}

const CUBES: [Cube; 14] = [
    ([-4., -10., -4.], [4., 0., 4.], [0., 0.], false, None, Part::Head),
    ([0., 0., 0.], [10., 2., 10.], [0., 64.], false, None, Part::Hat),
    ([0., 0., 0.], [7., 4., 7.], [0., 76.], false, None, Part::Hat2),
    ([0., 0., 0.], [4., 4., 4.], [0., 87.], false, None, Part::Hat3),
    // `CubeDeformation(0.25F)`.
    ([-0.25, -0.25, -0.25], [1.25, 2.25, 1.25], [0., 95.], false, Some([1., 2., 1.]), Part::Hat4),
    ([-1., -1., -6.], [1., 3., -4.], [24., 0.], false, None, Part::Nose),
    // `CubeDeformation(-0.25F)`.
    ([0.25, 3.25, -6.5], [0.75, 3.75, -6.0], [0., 0.], false, Some([1., 1., 1.]), Part::Mole),
    ([-4., 0., -3.], [4., 12., 3.], [16., 20.], false, None, Part::Body),
    // The jacket, `CubeDeformation(0.5F)`.
    ([-4.5, -0.5, -3.5], [4.5, 20.5, 3.5], [0., 38.], false, Some([8., 20., 6.]), Part::Body),
    ([-8., -2., -2.], [-4., 6., 2.], [44., 22.], false, None, Part::Arms),
    ([4., -2., -2.], [8., 6., 2.], [44., 22.], true, None, Part::Arms),
    ([-4., 2., -2.], [4., 6., 2.], [40., 38.], false, None, Part::Arms),
    ([-2., 0., -2.], [2., 12., 2.], [0., 22.], false, None, Part::RightLeg),
    ([-2., 0., -2.], [2., 12., 2.], [0., 22.], true, None, Part::LeftLeg),
];

/// `WitchModel.setupAnim`'s nose: a wobble at a pace set by the entity ID
/// (`0.01 * (id % 10)`), raised and tipped while it holds an item. Returns
/// its offset and pitch and roll.
pub fn nose_pose(entity_id: u64, age: f32, holding: bool) -> ([f32; 3], f32, f32) {
    let speed = 0.01 * (entity_id % 10) as f32;
    let x = (age * speed).sin() * 4.5 * (PI / 180.0);
    let z = (age * speed).cos() * 2.5 * (PI / 180.0);
    if holding {
        ([0.0, 1.0, -1.5], -0.9, z)
    } else {
        ([0.0, -2.0, 0.0], x, z)
    }
}

/// Appends the living witches.
pub fn append_witches<'a>(mesh: &mut ChunkMesh, witches: impl IntoIterator<Item = &'a WitchEntity>, walks: &WalkAnimations, atlas: &Atlas, light: &SkyLight, partial: f32) {
    let skin_id = ResourceId::parse("minecraft:entity/witch/witch").unwrap();
    if !atlas.contains(&skin_id) {
        return;
    }
    let skin = atlas.entity_region(&skin_id);
    let partial = partial.clamp(0.0, 1.0);
    for entity in witches {
        if entity.witch.health <= 0.0 {
            continue;
        }
        let feet = entity.previous_position.lerp(entity.witch.body.position, f64::from(partial));
        let eye = feet + glam::DVec3::new(0.0, f64::from(minecraftoss_entities::witch::EYE_HEIGHT), 0.0);
        let sample = (eye.x.floor() as i32, eye.y.floor() as i32, eye.z.floor() as i32);
        let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
        let body_yaw = entity.ai.body_rotation.body_yaw;
        let rotation = Quat::from_rotation_y(PI - body_yaw.to_radians());
        let look = &entity.ai.state.look_control;
        let walk = walks.get(entity.id);
        let (walk_position, walk_speed) = (walk.position(partial), walk.speed(partial));
        let age = entity.tick_count as f32 + partial;
        let head = Pose::ROOT.child([0.0; 3], look.pitch.to_radians(), (look.head_yaw - body_yaw).to_radians(), 0.0);
        let hat = head.child([-5.0, -10.03125, -5.0], 0.0, 0.0, 0.0);
        let hat2 = hat.child([1.75, -4.0, 2.0], -0.05235988, 0.0, 0.02617994);
        let hat3 = hat2.child([1.75, -4.0, 2.0], -0.10471976, 0.0, 0.05235988);
        let hat4 = hat3.child([1.75, -2.0, 2.0], -PI / 15.0, 0.0, 0.10471976);
        let (nose_offset, nose_x, nose_z) = nose_pose(entity.id, age, entity.witch.drinking.is_some());
        let nose = head.child(nose_offset, nose_x, 0.0, nose_z);
        let mole = nose.child([0.0, -2.0, 0.0], 0.0, 0.0, 0.0);
        let swing = walk_position * 0.6662;
        let right_leg = Pose::ROOT.child([-2.0, 12.0, 0.0], swing.cos() * 1.4 * walk_speed * 0.5, 0.0, 0.0);
        let left_leg = Pose::ROOT.child([2.0, 12.0, 0.0], (swing + PI).cos() * 1.4 * walk_speed * 0.5, 0.0, 0.0);
        let arms = Pose::ROOT.child([0.0, 3.0, -1.0], -0.75, 0.0, 0.0);
        for (from, to, uv, mirror, uv_size, part) in CUBES {
            let pose = match part {
                Part::Head => head,
                Part::Hat => hat,
                Part::Hat2 => hat2,
                Part::Hat3 => hat3,
                Part::Hat4 => hat4,
                Part::Nose => nose,
                Part::Mole => mole,
                Part::Body => Pose::ROOT,
                Part::Arms => arms,
                Part::RightLeg => right_leg,
                Part::LeftLeg => left_leg,
            };
            cube_tinted_pose_mirror(mesh, feet, rotation, 1.0, skin, sky, block, from, to, uv, pose.origin.to_array(), pose.rotation, [1.0; 3], [64., 128.], uv_size, mirror);
        }
    }
}

/// Appends each splash potion in flight as its item facing the camera
/// (`ThrownItemRenderer`, the ground display: half a block, raised an
/// eighth along the camera's up), the overlay tinted with its colour under
/// the bottle.
pub fn append_potions<'a>(mesh: &mut ChunkMesh, potions: impl IntoIterator<Item = (&'a PotionEntity, glam::DVec3)>, atlas: &Atlas, forward: Vec3, light: &SkyLight) {
    let (Ok(overlay), Ok(bottle)) = (ResourceId::parse("minecraft:item/potion_overlay"), ResourceId::parse("minecraft:item/splash_potion")) else { return };
    if !atlas.contains(&overlay) || !atlas.contains(&bottle) {
        return;
    }
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    for (entity, at) in potions {
        let position = at.as_vec3();
        let cell = (at.x.floor() as i32, at.y.floor() as i32, at.z.floor() as i32);
        let (sky, block) = (light.get(cell) as f32, light.get_block(cell) as f32);
        let rgb = entity.potion.potion.color();
        let tint = [((rgb >> 16) & 255) as f32 / 255.0, ((rgb >> 8) & 255) as f32 / 255.0, (rgb & 255) as f32 / 255.0, 1.0];
        let center = position + up * 0.125;
        for (layer, color, nudge) in [(&overlay, tint, 0.0_f32), (&bottle, [1.0; 4], 0.002)] {
            let [u0, v0, u1, v1] = atlas.region(layer);
            let start = mesh.vertices.len() as u32;
            for (corner, uv) in [(Vec2::new(-0.25, -0.25), [u0, v1]), (Vec2::new(0.25, -0.25), [u1, v1]), (Vec2::new(0.25, 0.25), [u1, v0]), (Vec2::new(-0.25, 0.25), [u0, v0])] {
                let point = center + right * corner.x + up * corner.y - forward * nudge;
                mesh.vertices.push(Vertex { position: point.to_array(), uv, color, sky_light: sky, block_light: block });
            }
            mesh.indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
            mesh.faces += 1;
        }
    }
}
