//! Pack-backed pinned 26.3 slime: `SlimeModel.createInnerBodyLayer` (a 6³
//! core with two eyes and a mouth, drawn cutout) under `SlimeOuterLayer`'s
//! 8³ shell (`createOuterBodyLayer`, `RenderTypes.entityTranslucent`, drawn
//! after the core), both from `entity/slime/slime` and sized by
//! `AbstractCubeMobRenderer.applySizeAndSquish` after `SlimeRenderer`'s
//! slight downscale: squashed wide on landing, stretched tall on take-off.
//! The death tip-over and hurt tint are not drawn.
use crate::{
    cow_render::cube_scaled,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::{Quat, Vec3};
use minecraftoss_entities::world::SlimeEntity;
use std::f32::consts::PI;

type Part = ([f32; 3], [f32; 3], [f32; 2]);

/// The core, right eye, left eye and mouth.
const INNER: [Part; 4] = [
    ([-3., 17., -3.], [3., 23., 3.], [0., 16.]),
    ([-3.25, 18., -3.5], [-1.25, 20., -1.5], [32., 0.]),
    ([1.25, 18., -3.5], [3.25, 20., -1.5], [32., 4.]),
    ([0., 21., -3.5], [1., 22., -2.5], [32., 8.]),
];
const OUTER: Part = ([-4., 16., -4.], [4., 24., 4.], [0., 0.]);

/// `SlimeRenderer.scale`: `downscaleSlightly` (0.999), then
/// `applySizeAndSquish`, whose squash `w` shrinks with the size.
pub fn size_and_squish(size: i32, squish: f32) -> Vec3 {
    let size = size as f32;
    let ss = squish / (size * 0.5 + 1.0);
    let w = 1.0 / (ss + 1.0);
    Vec3::new(w * size, 1.0 / w * size, w * size) * 0.999
}

pub fn append_slimes<'a>(
    mesh: &mut ChunkMesh,
    translucent: &mut ChunkMesh,
    slimes: impl IntoIterator<Item = &'a SlimeEntity>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let region = atlas.entity_region(&ResourceId::parse("minecraft:entity/slime/slime").unwrap());
    let partial = partial.clamp(0.0, 1.0);
    for entity in slimes {
        let slime = &entity.slime;
        if slime.health <= 0.0 {
            continue;
        }
        let feet = entity.previous_position.lerp(slime.body.position, f64::from(partial));
        let sample = (feet.x.floor() as i32, (feet.y + f64::from(slime.eye_height())).floor() as i32, feet.z.floor() as i32);
        let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
        let rotation = Quat::from_rotation_y(PI - entity.ai.body_rotation.body_yaw.to_radians());
        // `extractRenderState`: the squash between ticks.
        let squish = slime.previous_squish + (slime.squish - slime.previous_squish) * partial;
        let scale = size_and_squish(slime.size, squish);
        for (from, to, uv) in INNER {
            cube_scaled(mesh, feet, rotation, scale, region, sky, block, from, to, uv, [0.0; 3], Quat::IDENTITY, [1.0; 3], [64., 32.], None, false);
        }
        let (from, to, uv) = OUTER;
        cube_scaled(translucent, feet, rotation, scale, region, sky, block, from, to, uv, [0.0; 3], Quat::IDENTITY, [1.0; 3], [64., 32.], None, false);
    }
}
