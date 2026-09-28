//! Pack-backed bat geometry from pinned 26.3 BatModel.createBodyLayer.
//! Bone animation and exact per-part resting/flying poses remain to be added.
use crate::{
    cow_render::cube_tinted,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::{DVec3, Quat};
use minecraftoss_entities::world::BatEntity;

pub fn append_bats<'a>(
    mesh: &mut ChunkMesh,
    bats: impl IntoIterator<Item = &'a BatEntity>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let id = ResourceId::parse("minecraft:entity/bat/bat").unwrap();
    let region = atlas.entity_region(&id);
    for entity in bats {
        if entity.bat.health <= 0.0 {
            continue;
        }
        let feet = entity
            .previous_position
            .lerp(entity.bat.body.position, partial.clamp(0.0, 1.0) as f64);
        let (origin, rotation) = if entity.bat.resting {
            (
                feet + DVec3::Y * 0.9,
                Quat::from_rotation_z(std::f32::consts::PI),
            )
        } else {
            (feet, Quat::IDENTITY)
        };
        let sample = (
            feet.x.floor() as i32,
            (feet.y + 0.45).floor() as i32,
            feet.z.floor() as i32,
        );
        let sky = light.get(sample) as f32;
        let block = light.get_block(sample) as f32;
        // Each box uses BatModel's local coordinates, UV origin and part offset.
        for (from, to, uv, pivot) in [
            ([-1.5, 0., -1.], [1.5, 5., 1.], [0., 0.], [0., 17., 0.]),
            ([-2., -3., -1.], [2., 0., 1.], [0., 7.], [0., 17., 0.]),
            ([-2.5, -4., 0.], [0.5, 1., 0.], [1., 15.], [-1.5, 15., 0.]),
            ([-0.1, -3., 0.], [2.9, 2., 0.], [8., 15.], [1.1, 14., 0.]),
            ([-2., -2., 0.], [0., 5., 0.], [12., 0.], [-1.5, 17., 0.]),
            ([-6., -2., 0.], [0., 6., 0.], [16., 0.], [-3.5, 17., 0.]),
            ([0., -2., 0.], [2., 5., 0.], [12., 7.], [1.5, 17., 0.]),
            ([0., -2., 0.], [6., 6., 0.], [16., 8.], [3.5, 17., 0.]),
            ([-1.5, 0., 0.], [1.5, 2., 0.], [16., 16.], [0., 22., 0.]),
        ] {
            cube_tinted(
                mesh,
                origin,
                rotation,
                1.0,
                region,
                sky,
                block,
                from,
                to,
                uv,
                pivot,
                false,
                [1.0; 3],
                [32., 32.],
                None,
            );
        }
    }
}
