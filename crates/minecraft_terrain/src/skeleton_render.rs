//! Pack-backed pinned 26.3 skeleton body layers.
//! Source: SkeletonModel.createBodyLayer/createDefaultSkeletonMesh and
//! createSingleModelDualBodyLayer (the parched), BoggedModel's mushrooms,
//! HumanoidModel.createMesh/poseRightArm, SkeletonClothingLayer (the stray's
//! and bogged's outer layers, `LayerDefinitions` inflating the humanoid mesh
//! by 0.25 and 0.2), and each renderer's textures.
use crate::{
    cow_render::cube_tinted_pose_mirror,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::{EulerRot, Quat, Vec3};
use minecraftoss_entities::{skeleton::SkeletonKind, world::SkeletonEntity};

/// A cuboid: corners, texture offset, pivot, mirrored, the size its UVs
/// come from when inflated, and which pose it takes (0 body, 1 head,
/// 2 right arm, 3 left arm, 4 legs).
type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3], bool, Option<[f32; 3]>, u8);

/// `createDefaultSkeletonMesh` over `HumanoidModel.createMesh`.
const SKELETON: [Part; 7] = [
    ([-4., -8., -4.], [4., 0., 4.], [0., 0.], [0., 0., 0.], false, None, 1),
    ([-4.5, -8.5, -4.5], [4.5, 0.5, 4.5], [32., 0.], [0., 0., 0.], false, Some([8., 8., 8.]), 1),
    ([-4., 0., -2.], [4., 12., 2.], [16., 16.], [0., 0., 0.], false, None, 0),
    ([-1., -2., -1.], [1., 10., 1.], [40., 16.], [-5., 2., 0.], false, None, 2),
    ([-1., -2., -1.], [1., 10., 1.], [40., 16.], [5., 2., 0.], true, None, 3),
    ([-1., 0., -1.], [1., 12., 1.], [0., 16.], [-2., 12., 0.], false, None, 4),
    ([-1., 0., -1.], [1., 12., 1.], [0., 16.], [2., 12., 0.], true, None, 4),
];

/// `createSingleModelDualBodyLayer`: each part with its outer box, on one
/// 64×64 sheet.
const PARCHED: [Part; 13] = [
    ([-4., 0., -2.], [4., 12., 2.], [16., 16.], [0., 0., 0.], false, None, 0),
    ([-4., 10., -2.], [4., 11., 2.], [28., 0.], [0., 0., 0.], false, None, 0),
    ([-4.025, -0.025, -2.025], [4.025, 12.025, 2.025], [16., 48.], [0., 0., 0.], false, Some([8., 12., 4.]), 0),
    ([-4., -8., -4.], [4., 0., 4.], [0., 0.], [0., 0., 0.], false, None, 1),
    ([-4.2, -8.2, -4.2], [4.2, 0.2, 4.2], [0., 32.], [0., 0., 0.], false, Some([8., 8., 8.]), 1),
    ([-1., -2., -1.], [1., 10., 1.], [40., 16.], [-5.5, 2., 0.], false, None, 2),
    ([-1.55, -2.025, -1.5], [1.45, 9.975, 1.5], [42., 33.], [-5.5, 2., 0.], false, None, 2),
    ([-1., -2., -1.], [1., 10., 1.], [56., 16.], [5.5, 2., 0.], false, None, 3),
    ([-1.45, -2.025, -1.5], [1.55, 9.975, 1.5], [40., 48.], [5.5, 2., 0.], false, None, 3),
    ([-1., 0., -1.], [1., 12., 1.], [0., 16.], [-2., 12., 0.], false, None, 4),
    ([-1.5, 0., -1.5], [1.5, 12., 1.5], [0., 49.], [-2., 12., 0.], false, None, 4),
    ([-1., 0., -1.], [1., 12., 1.], [0., 16.], [2., 12., 0.], false, None, 4),
    ([-1.5, 0., -1.5], [1.5, 12., 1.5], [4., 49.], [2., 12., 0.], false, None, 4),
];

/// `HumanoidModel.createMesh` inflated by `grow`: the clothing layer's
/// parts (the right leg only; the left one follows mirrored).
fn clothing(grow: f32) -> [Part; 7] {
    let box_ = |from: [f32; 3], size: [f32; 3], extra: f32| -> ([f32; 3], [f32; 3]) {
        let g = grow + extra;
        ([from[0] - g, from[1] - g, from[2] - g], [from[0] + size[0] + g, from[1] + size[1] + g, from[2] + size[2] + g])
    };
    let (head_from, head_to) = box_([-4., -8., -4.], [8., 8., 8.], 0.0);
    let (hat_from, hat_to) = box_([-4., -8., -4.], [8., 8., 8.], 0.5);
    let (body_from, body_to) = box_([-4., 0., -2.], [8., 12., 4.], 0.0);
    let (right_arm_from, right_arm_to) = box_([-3., -2., -2.], [4., 12., 4.], 0.0);
    let (left_arm_from, left_arm_to) = box_([-1., -2., -2.], [4., 12., 4.], 0.0);
    let (leg_from, leg_to) = box_([-2., 0., -2.], [4., 12., 4.], 0.0);
    [
        (head_from, head_to, [0., 0.], [0., 0., 0.], false, Some([8., 8., 8.]), 1),
        (hat_from, hat_to, [32., 0.], [0., 0., 0.], false, Some([8., 8., 8.]), 1),
        (body_from, body_to, [16., 16.], [0., 0., 0.], false, Some([8., 12., 4.]), 0),
        (right_arm_from, right_arm_to, [40., 16.], [-5., 2., 0.], false, Some([4., 12., 4.]), 2),
        (left_arm_from, left_arm_to, [40., 16.], [5., 2., 0.], true, Some([4., 12., 4.]), 3),
        (leg_from, leg_to, [0., 16.], [-1.9, 12., 0.], false, Some([4., 12., 4.]), 4),
        (leg_from, leg_to, [0., 16.], [1.9, 12., 0.], true, Some([4., 12., 4.]), 4),
    ]
}

/// `BoggedModel`'s mushrooms, children of the head: box, texture offset,
/// offset and rotation (x, y, z).
const MUSHROOMS: [([f32; 3], [f32; 3], [f32; 2], [f32; 3], [f32; 3]); 6] = [
    ([-3., -3., 0.], [3., 1., 0.], [50., 16.], [3., -8., 3.], [0., std::f32::consts::FRAC_PI_4, 0.]),
    ([-3., -3., 0.], [3., 1., 0.], [50., 16.], [3., -8., 3.], [0., 3.0 * std::f32::consts::FRAC_PI_4, 0.]),
    ([-3., -3., 0.], [3., 1., 0.], [50., 22.], [-3., -8., -3.], [0., std::f32::consts::FRAC_PI_4, 0.]),
    ([-3., -3., 0.], [3., 1., 0.], [50., 22.], [-3., -8., -3.], [0., 3.0 * std::f32::consts::FRAC_PI_4, 0.]),
    ([-3., -4., 0.], [3., 0., 0.], [50., 28.], [-2., -1., 4.], [-std::f32::consts::FRAC_PI_2, 0., std::f32::consts::FRAC_PI_4]),
    ([-3., -4., 0.], [3., 0., 0.], [50., 28.], [-2., -1., 4.], [-std::f32::consts::FRAC_PI_2, 0., 3.0 * std::f32::consts::FRAC_PI_4]),
];

fn texture(path: &str) -> ResourceId {
    ResourceId::parse(&format!("minecraft:entity/skeleton/{path}")).unwrap()
}

pub fn append_skeletons<'a>(
    mesh: &mut ChunkMesh,
    skeletons: impl IntoIterator<Item = &'a SkeletonEntity>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    for entity in skeletons {
        if entity.skeleton.health <= 0.0 {
            continue;
        }
        let kind = entity.skeleton.kind;
        let feet = entity.previous_position.lerp(
            entity.skeleton.body.position,
            partial.clamp(0.0, 1.0) as f64,
        );
        let sample = (
            feet.x.floor() as i32,
            (feet.y + f64::from(entity.skeleton.eye_height())).floor() as i32,
            feet.z.floor() as i32,
        );
        let sky = light.get(sample) as f32;
        let block = light.get_block(sample) as f32;
        let body_yaw = entity.body_rotation.body_yaw;
        let rotation = Quat::from_rotation_y(std::f32::consts::PI - body_yaw.to_radians());
        let head_yaw = (entity.look_control.head_yaw - body_yaw).to_radians();
        let head_pitch = entity.look_control.pitch.to_radians();
        let head = Quat::from_euler(EulerRot::ZYX, 0.0, head_yaw, head_pitch);
        // `HumanoidModel.poseRightArm`'s bow pose while aggressive, else the
        // skeleton's resting right arm.
        let pose_of = |pose: u8| match pose {
            1 => head,
            2 if entity.bow.aggressive => Quat::from_euler(EulerRot::ZYX, 0.0, -0.1 + head_yaw, -std::f32::consts::FRAC_PI_2 + head_pitch),
            3 if entity.bow.aggressive => Quat::from_euler(EulerRot::ZYX, 0.0, 0.5 + head_yaw, -std::f32::consts::FRAC_PI_2 + head_pitch),
            2 => Quat::from_rotation_x(-std::f32::consts::PI / 10.0),
            _ => Quat::IDENTITY,
        };
        let mut draw = |parts: &[Part], id: &ResourceId, size: [f32; 2]| {
            let region = atlas.entity_region(id);
            for (from, to, uv, pivot, mirror, uv_size, pose) in parts {
                cube_tinted_pose_mirror(mesh, feet, rotation, 1.0, region, sky, block, *from, *to, *uv, *pivot, pose_of(*pose), [1.0; 3], size, *uv_size, *mirror);
            }
        };
        match kind {
            SkeletonKind::Parched => draw(&PARCHED, &texture("parched"), [64., 64.]),
            SkeletonKind::Skeleton => draw(&SKELETON, &texture("skeleton"), [64., 32.]),
            SkeletonKind::Stray => {
                draw(&SKELETON, &texture("stray"), [64., 32.]);
                draw(&clothing(0.25), &texture("stray_overlay"), [64., 32.]);
            }
            SkeletonKind::Bogged => {
                draw(&SKELETON, &texture("bogged"), [64., 32.]);
                draw(&clothing(0.2), &texture("bogged_overlay"), [64., 32.]);
            }
        }
        // `BoggedModel.setupAnim`: the mushrooms show until sheared.
        if kind == SkeletonKind::Bogged && !entity.skeleton.sheared {
            let region = atlas.entity_region(&texture("bogged"));
            for (from, to, uv, offset, [x, y, z]) in MUSHROOMS {
                let pivot = head * Vec3::from_array(offset);
                let pose = head * Quat::from_euler(EulerRot::ZYX, z, y, x);
                cube_tinted_pose_mirror(mesh, feet, rotation, 1.0, region, sky, block, from, to, uv, pivot.to_array(), pose, [1.0; 3], [64., 32.], None, false);
            }
        }
    }
}
