//! Pack-backed pinned 26.3 zombie adult/baby cuboids.
//! Source: HumanoidModel.createMesh, BabyZombieModel.createBodyLayer,
//! LayerDefinitions.ZOMBIE and AbstractZombieRenderer textures; husks use
//! the zombie's model (`HuskRenderer`), zombie villagers
//! `ZombieVillagerModel`/`BabyZombieVillagerModel` with the villager type
//! and profession overlays (`VillagerProfessionLayer`).
//! The body faces its body yaw and the head its look. Walk/swing
//! animation, mirrored UVs and equipment overlays remain open.
use crate::{
    cow_render::cube_tinted_pose,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::{EulerRot, Quat};
use minecraftoss_entities::world::ZombieEntity;
use minecraftoss_entities::zombie::ZombieKind;

type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3]);

const ADULT: [Part; 7] = [
    ([-4., -8., -4.], [4., 0., 4.], [0., 0.], [0., 0., 0.]),
    ([-4.5, -8.5, -4.5], [4.5, 0.5, 4.5], [32., 0.], [0., 0., 0.]),
    ([-4., 0., -2.], [4., 12., 2.], [16., 16.], [0., 0., 0.]),
    ([-3., -2., -2.], [1., 10., 2.], [40., 16.], [-5., 2., 0.]),
    ([-1., -2., -2.], [3., 10., 2.], [40., 16.], [5., 2., 0.]),
    ([-2., 0., -2.], [2., 12., 2.], [0., 16.], [-1.9, 12., 0.]),
    ([-2., 0., -2.], [2., 12., 2.], [0., 16.], [1.9, 12., 0.]),
];

const BABY: [Part; 7] = [
    (
        [-3., -6.25, -3.],
        [3., -0.25, 3.],
        [3., 3.],
        [0., 15.25, 0.],
    ),
    (
        [-3.25, -6.4, -3.25],
        [3.25, 0.1, 3.25],
        [35., 3.],
        [0., 15.25, 0.],
    ),
    ([-2., -2.5, -1.], [2., 2.5, 1.], [16., 16.], [0., 17.5, 0.]),
    ([-1., -0.5, -1.], [1., 4.5, 1.], [36., 16.], [-3., 15.5, 0.]),
    ([-1., -0.5, -1.], [1., 4.5, 1.], [28., 16.], [3., 15.5, 0.]),
    ([-1., 0., -1.], [1., 4., 1.], [8., 16.], [-1., 20., 0.]),
    ([-1., 0., -1.], [1., 4., 1.], [0., 16.], [1., 20., 0.]),
];

pub fn append_zombies<'a>(
    mesh: &mut ChunkMesh,
    zombies: impl IntoIterator<Item = &'a ZombieEntity>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    for entity in zombies {
        if entity.zombie.health <= 0.0 {
            continue;
        }
        let zombie = &entity.zombie;
        let feet = entity
            .previous_position
            .lerp(zombie.body.position, partial.clamp(0.0, 1.0) as f64);
        if zombie.kind == ZombieKind::ZombieVillager {
            append_zombie_villager(mesh, entity, feet, atlas, light);
            continue;
        }
        let id = ResourceId::parse(match (zombie.kind, zombie.baby) {
            (ZombieKind::Zombie, false) => "minecraft:entity/zombie/zombie",
            (ZombieKind::Zombie, true) => "minecraft:entity/zombie/zombie_baby",
            (ZombieKind::Drowned, false) => "minecraft:entity/zombie/drowned",
            (ZombieKind::Drowned, true) => "minecraft:entity/zombie/drowned_baby",
            (ZombieKind::Husk, false) => "minecraft:entity/zombie/husk",
            (ZombieKind::Husk, true) => "minecraft:entity/zombie/husk_baby",
            (ZombieKind::ZombieVillager, _) => unreachable!("drawn by append_zombie_villager"),
        })
        .unwrap();
        let region = atlas.entity_region(&id);
        let sample = (
            feet.x.floor() as i32,
            (feet.y + f64::from(zombie.eye_height())).floor() as i32,
            feet.z.floor() as i32,
        );
        let sky = light.get(sample) as f32;
        let block = light.get_block(sample) as f32;
        let body_yaw = entity.body_rotation.body_yaw;
        let rotation = Quat::from_rotation_y(std::f32::consts::PI - body_yaw.to_radians());
        let head = Quat::from_euler(EulerRot::ZYX, 0.0, (entity.look_control.head_yaw - body_yaw).to_radians(), entity.look_control.pitch.to_radians());
        for (index, (from, to, uv, pivot)) in (if zombie.baby { &BABY } else { &ADULT })
            .iter()
            .enumerate()
        {
            // AnimationUtils.animateZombieArms gives the idle zombie its
            // characteristic raised arms even before a target is acquired.
            // Swing animation is not yet represented in the viewer state.
            let part_rotation = if index == 3 || index == 4 {
                let x = -std::f32::consts::PI / if entity.aggressive { 1.5 } else { 2.25 };
                let y = if index == 3 { -0.1 } else { 0.1 };
                Quat::from_euler(EulerRot::ZYX, 0.0, y, x)
            } else if index <= 1 {
                head
            } else {
                Quat::IDENTITY
            };
            cube_tinted_pose(
                mesh,
                feet,
                rotation,
                1.0,
                region,
                sky,
                block,
                *from,
                *to,
                *uv,
                *pivot,
                part_rotation,
                [1.0; 3],
                [64., 64.],
                // The hat is inflated, but ModelPart.Cube builds its UVs
                // from the original size before CubeDeformation is applied.
                if index == 1 {
                    Some(if zombie.baby {
                        [6., 6., 6.]
                    } else {
                        [8., 8., 8.]
                    })
                } else {
                    None
                },
            );
        }
    }
}

/// A zombie villager part: box corners, texture offset, pivot, which pose
/// it takes (0 body, 1 head, 2 right arm, 3 left arm, 4 the head's hat
/// rim), mirrored texture and the size its UVs come from when inflated.
type VillagerPart = ([f32; 3], [f32; 3], [f32; 2], [f32; 3], u8, bool, Option<[f32; 3]>);

/// `ZombieVillagerModel.createBodyLayer`: the head with its nose, hat and
/// rim, the body with its coat, zombie arms and legs.
const VILLAGER_ADULT: [VillagerPart; 10] = [
    ([-4., -10., -4.], [4., 0., 4.], [0., 0.], [0., 0., 0.], 1, false, None),
    ([-1., -3., -6.], [1., 1., -4.], [24., 0.], [0., 0., 0.], 1, false, None),
    ([-4.5, -10.5, -4.5], [4.5, 0.5, 4.5], [32., 0.], [0., 0., 0.], 1, false, Some([8., 10., 8.])),
    ([-8., -8., -6.], [8., 8., -5.], [30., 47.], [0., 0., 0.], 4, false, None),
    ([-4., 0., -3.], [4., 12., 3.], [16., 20.], [0., 0., 0.], 0, false, None),
    ([-4.05, -0.05, -3.05], [4.05, 20.05, 3.05], [0., 38.], [0., 0., 0.], 0, false, Some([8., 20., 6.])),
    ([-3., -2., -2.], [1., 10., 2.], [44., 22.], [-5., 2., 0.], 2, false, None),
    ([-1., -2., -2.], [3., 10., 2.], [44., 22.], [5., 2., 0.], 3, true, None),
    ([-2., 0., -2.], [2., 12., 2.], [0., 22.], [-2., 12., 0.], 0, false, None),
    ([-2., 0., -2.], [2., 12., 2.], [0., 22.], [2., 12., 0.], 0, true, None),
];

/// `BabyZombieVillagerModel.createBodyLayer`, children folded into the
/// head's space.
const VILLAGER_BABY: [VillagerPart; 10] = [
    ([-2., -2.75, -1.5], [2., 2.25, 1.5], [0., 15.], [0., 18.75, 0.], 0, false, None),
    ([-2.1, -2.85, -1.6], [2.1, 3.35, 1.6], [16., 22.], [0., 18.75, 0.], 0, false, Some([4., 6., 3.])),
    ([-4., -8., -3.5], [4., 0., 3.5], [0., 0.], [0., 16., 0.], 1, false, None),
    ([-4.3, -8.3, -3.8], [4.3, 0.3, 3.8], [0., 31.], [0., 16., 0.], 1, false, Some([8., 8., 7.])),
    ([-7., -5., -6.], [7., -4., 6.], [0., 46.], [0., 16., 0.], 1, false, None),
    ([-1., -2., -4.5], [1., 0., -3.5], [23., 0.], [0., 16., 0.], 1, false, None),
    ([-1., -0.5, -1.], [1., 4.5, 1.], [24., 15.], [-3., 15.5, 0.], 2, false, None),
    ([-1., -0.5, -1.], [1., 4.5, 1.], [16., 15.], [3., 15.5, 0.], 3, false, None),
    ([-1., -0.5, -1.], [1., 2.5, 1.], [8., 23.], [-1., 21.5, 0.], 0, false, None),
    ([-1., -0.5, -1.], [1., 2.5, 1.], [0., 23.], [1., 21.5, 0.], 0, false, None),
];

/// The part of a namespaced ID after the namespace.
fn path_of(id: &str) -> &str {
    id.split_once(':').map_or(id, |(_, path)| path)
}

/// A zombie villager: its base skin, then its villager type's overlay and,
/// for adults, its profession's, on the same model.
fn append_zombie_villager(mesh: &mut ChunkMesh, entity: &ZombieEntity, feet: glam::DVec3, atlas: &Atlas, light: &SkyLight) {
    let zombie = &entity.zombie;
    let sample = (feet.x.floor() as i32, (feet.y + f64::from(zombie.eye_height())).floor() as i32, feet.z.floor() as i32);
    let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
    let body_yaw = entity.body_rotation.body_yaw;
    let rotation = Quat::from_rotation_y(std::f32::consts::PI - body_yaw.to_radians());
    let head = Quat::from_euler(EulerRot::ZYX, 0.0, (entity.look_control.head_yaw - body_yaw).to_radians(), entity.look_control.pitch.to_radians());
    // `AnimationUtils.animateZombieArms` without a swing: raised arms,
    // higher when aggressive.
    let arm_x = -std::f32::consts::PI / if entity.aggressive { 1.5 } else { 2.25 };
    let right_arm = Quat::from_euler(EulerRot::ZYX, 0.0, -0.1, arm_x);
    let left_arm = Quat::from_euler(EulerRot::ZYX, 0.0, 0.1, arm_x);
    let rim = head * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let (kind, profession) = zombie.villager.as_ref().map_or(("plains", "none"), |(kind, profession)| (path_of(kind), path_of(profession)));
    let mut skins = vec![if zombie.baby { "minecraft:entity/zombie_villager/zombie_villager_baby".to_owned() } else { "minecraft:entity/zombie_villager/zombie_villager".to_owned() }];
    skins.push(format!("minecraft:entity/zombie_villager/{}/{kind}", if zombie.baby { "baby" } else { "type" }));
    if !zombie.baby && profession != "none" {
        skins.push(format!("minecraft:entity/zombie_villager/profession/{profession}"));
    }
    let parts = if zombie.baby { &VILLAGER_BABY } else { &VILLAGER_ADULT };
    for skin in skins {
        let Ok(id) = ResourceId::parse(&skin) else { continue };
        if !atlas.contains(&id) {
            continue;
        }
        let region = atlas.entity_region(&id);
        for &(from, to, uv, pivot, pose, mirror, dims) in parts {
            let part_rotation = match pose {
                1 => head,
                2 => right_arm,
                3 => left_arm,
                4 => rim,
                _ => Quat::IDENTITY,
            };
            crate::cow_render::cube_scaled(mesh, feet, rotation, glam::Vec3::ONE, region, sky, block, from, to, uv, pivot, part_rotation, [1.0; 3], [64., 64.], dims, mirror);
        }
    }
}
