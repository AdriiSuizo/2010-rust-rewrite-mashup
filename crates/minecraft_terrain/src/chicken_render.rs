//! Pinned 26.3 AdultChickenModel, BabyChickenModel and ColdChickenModel cuboids.
//! Textures resolve through the selected resource-pack atlas.
use crate::{
    cow_render::cube_tinted,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::Quat;
use minecraftoss_entities::{chicken::ChickenVariant, world::ChickenEntity};

type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3], bool);

const ADULT: [Part; 8] = [
    (
        [-2., -6., -2.],
        [2., 0., 1.],
        [0., 0.],
        [0., 15., -4.],
        false,
    ),
    (
        [-2., -4., -4.],
        [2., -2., -2.],
        [14., 0.],
        [0., 15., -4.],
        false,
    ),
    (
        [-1., -2., -3.],
        [1., 0., -1.],
        [14., 4.],
        [0., 15., -4.],
        false,
    ),
    ([-3., -4., -3.], [3., 4., 3.], [0., 9.], [0., 16., 0.], true),
    (
        [-1., 0., -3.],
        [2., 5., 0.],
        [26., 0.],
        [-2., 19., 1.],
        false,
    ),
    (
        [-1., 0., -3.],
        [2., 5., 0.],
        [26., 0.],
        [1., 19., 1.],
        false,
    ),
    (
        [0., 0., -3.],
        [1., 4., 3.],
        [24., 13.],
        [-4., 13., 0.],
        false,
    ),
    (
        [-1., 0., -3.],
        [0., 4., 3.],
        [24., 13.],
        [4., 13., 0.],
        false,
    ),
];

const BABY: [Part; 8] = [
    (
        [-2., -2.25, -0.75],
        [2., 1.75, 3.25],
        [0., 0.],
        [0., 20.25, -1.25],
        false,
    ),
    (
        [-1., -0.25, -1.75],
        [1., 0.75, -0.75],
        [10., 8.],
        [0., 20.25, -1.25],
        false,
    ),
    (
        [-0.5, 0., 0.],
        [0.5, 2., 0.],
        [2., 2.],
        [1., 22., 0.5],
        false,
    ),
    (
        [-0.5, 2., -1.],
        [0.5, 2., 0.],
        [0., 1.],
        [1., 22., 0.5],
        false,
    ),
    (
        [-0.5, 0., 0.],
        [0.5, 2., 0.],
        [0., 2.],
        [-1., 22., 0.5],
        false,
    ),
    (
        [-0.5, 2., -1.],
        [0.5, 2., 0.],
        [0., 0.],
        [-1., 22., 0.5],
        false,
    ),
    ([0., 0., -1.], [1., 0., 1.], [6., 8.], [2., 20., 0.], false),
    (
        [-1., 0., -1.],
        [0., 0., 1.],
        [4., 8.],
        [-2., 20., 0.],
        false,
    ),
];

pub fn texture_id(variant: ChickenVariant, baby: bool) -> ResourceId {
    let kind = match variant {
        ChickenVariant::Temperate => "temperate",
        ChickenVariant::Warm => "warm",
        ChickenVariant::Cold => "cold",
    };
    ResourceId::parse(&format!(
        "minecraft:entity/chicken/chicken_{kind}{}",
        if baby { "_baby" } else { "" }
    ))
    .unwrap()
}

pub fn append_chickens<'a>(
    mesh: &mut ChunkMesh,
    chickens: impl IntoIterator<Item = &'a ChickenEntity>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    for entity in chickens {
        let chicken = &entity.chicken;
        if chicken.health <= 0.0 {
            continue;
        }
        let feet = entity
            .previous_position
            .lerp(chicken.body.position, partial.clamp(0.0, 1.0) as f64);
        let yaw_delta = (chicken.yaw - entity.previous_yaw + 180.0).rem_euclid(360.0) - 180.0;
        let yaw = entity.previous_yaw + yaw_delta * partial.clamp(0.0, 1.0);
        let rotation = Quat::from_rotation_y(std::f32::consts::PI - yaw.to_radians());
        let baby = chicken.age.baby();
        let region = atlas.entity_region(&texture_id(chicken.variant, baby));
        let pos = (
            feet.x.floor() as i32,
            (feet.y + 0.35).floor() as i32,
            feet.z.floor() as i32,
        );
        let sky = light.get(pos) as f32;
        let block = light.get_block(pos) as f32;
        for &(from, to, uv, pivot, rotate_body) in if baby { &BABY[..] } else { &ADULT[..] } {
            cube_tinted(
                mesh,
                feet,
                rotation,
                1.0,
                region,
                sky,
                block,
                from,
                to,
                uv,
                pivot,
                rotate_body,
                [1.0; 3],
                if baby { [16., 16.] } else { [64., 32.] },
                None,
            );
        }
        if chicken.variant == ChickenVariant::Cold && !baby {
            for (from, to, uv, pivot, rotate_body) in [
                ([0., 3., -1.], [0., 6., 4.], [38., 9.], [0., 16., 0.], true),
                (
                    [-3., -7., -2.015],
                    [3., -4., 1.985],
                    [44., 0.],
                    [0., 15., -4.],
                    false,
                ),
            ] {
                cube_tinted(
                    mesh,
                    feet,
                    rotation,
                    1.0,
                    region,
                    sky,
                    block,
                    from,
                    to,
                    uv,
                    pivot,
                    rotate_body,
                    [1.0; 3],
                    [64., 32.],
                    None,
                );
            }
        }
    }
}
