//! Pinned 26.3 PigModel/BabyPigModel cuboids with pack-resolved textures.
//! Active gait and view-dependent pose remain separate.
use crate::{
    cow_render::cube_tinted,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::Quat;
use minecraftoss_entities::{pig::PigVariant, world::PigEntity};

type BoxPart = (
    [f32; 3],
    [f32; 3],
    [f32; 2],
    [f32; 3],
    bool,
    Option<[f32; 3]>,
);

const ADULT: [BoxPart; 7] = [
    (
        [-4., -4., -8.],
        [4., 4., 0.],
        [0., 0.],
        [0., 12., -6.],
        false,
        None,
    ),
    (
        [-2., 0., -9.],
        [2., 3., -8.],
        [16., 16.],
        [0., 12., -6.],
        false,
        None,
    ),
    (
        [-5., -10., -7.],
        [5., 6., 1.],
        [28., 8.],
        [0., 11., 2.],
        true,
        None,
    ),
    (
        [-2., 0., -2.],
        [2., 6., 2.],
        [0., 16.],
        [-3., 18., 7.],
        false,
        None,
    ),
    (
        [-2., 0., -2.],
        [2., 6., 2.],
        [0., 16.],
        [3., 18., 7.],
        false,
        None,
    ),
    (
        [-2., 0., -2.],
        [2., 6., 2.],
        [0., 16.],
        [-3., 18., -5.],
        false,
        None,
    ),
    (
        [-2., 0., -2.],
        [2., 6., 2.],
        [0., 16.],
        [3., 18., -5.],
        false,
        None,
    ),
];

const BABY: [BoxPart; 7] = [
    (
        [-3.5, -3., -4.5],
        [3.5, 3., 4.5],
        [0., 0.],
        [0., 19., 0.5],
        false,
        None,
    ),
    (
        [-3.525, -5.025, -5.025],
        [3.525, 1.025, 1.025],
        [0., 15.],
        [0., 19., -2.],
        false,
        Some([7., 6., 6.]),
    ),
    (
        [-1.515, -1.99, -6.015],
        [1.515, 0.04, -4.985],
        [6., 27.],
        [0., 19., -2.],
        false,
        Some([3., 2., 1.]),
    ),
    (
        [-1., 0., -1.],
        [1., 2., 1.],
        [0., 0.],
        [2.5, 22., -3.],
        false,
        None,
    ),
    (
        [-1., 0., -1.],
        [1., 2., 1.],
        [23., 0.],
        [-2.5, 22., -3.],
        false,
        None,
    ),
    (
        [-1., 0., -1.],
        [1., 2., 1.],
        [0., 4.],
        [2.5, 22., 4.],
        false,
        None,
    ),
    (
        [-1., 0., -1.],
        [1., 2., 1.],
        [23., 4.],
        [-2.5, 22., 4.],
        false,
        None,
    ),
];

pub fn texture_id(variant: PigVariant, baby: bool) -> ResourceId {
    let kind = match variant {
        PigVariant::Temperate => "temperate",
        PigVariant::Warm => "warm",
        PigVariant::Cold => "cold",
    };
    ResourceId::parse(&format!(
        "minecraft:entity/pig/pig_{kind}{}",
        if baby { "_baby" } else { "" }
    ))
    .unwrap()
}

pub fn append_pigs<'a>(mesh: &mut ChunkMesh, pigs: impl IntoIterator<Item = &'a PigEntity>, atlas: &Atlas, light: &SkyLight) {
    for entity in pigs {
        let pig = &entity.pig;
        if pig.health <= 0.0 {
            continue;
        }
        let feet = pig.body.position;
        let baby = pig.age.baby();
        let region = atlas.entity_region(&texture_id(pig.variant, baby));
        let position = (
            feet.x.floor() as i32,
            (feet.y + 0.5).floor() as i32,
            feet.z.floor() as i32,
        );
        let sky = light.get(position) as f32;
        let block = light.get_block(position) as f32;
        for &(from, to, uv, pivot, rotate_body, dimensions) in
            if baby { &BABY[..] } else { &ADULT[..] }
        {
            cube_tinted(
                mesh,
                feet,
                Quat::from_rotation_y(std::f32::consts::PI),
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
                if baby { [32., 32.] } else { [64., 64.] },
                dimensions,
            );
        }
        if pig.variant == PigVariant::Cold && !baby {
            // ColdPigModel adds a half-pixel body coat over the base body.
            cube_tinted(
                mesh,
                feet,
                Quat::from_rotation_y(std::f32::consts::PI),
                1.0,
                region,
                sky,
                block,
                [-5.5, -10.5, -7.5],
                [5.5, 6.5, 1.5],
                [28., 32.],
                [0., 11., 2.],
                true,
                [1.0; 3],
                [64., 64.],
                Some([10., 16., 8.]),
            );
        }
        if pig.saddled && !baby {
            // ModelLayers.PIG_SADDLE bakes PigModel with CubeDeformation(0.5).
            let saddle = atlas.entity_region(
                &ResourceId::parse("minecraft:entity/equipment/pig_saddle/saddle").unwrap(),
            );
            for &(from, to, uv, pivot, rotate_body, _) in &ADULT {
                let dimensions = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
                let inflated_from = [from[0] - 0.5, from[1] - 0.5, from[2] - 0.5];
                let inflated_to = [to[0] + 0.5, to[1] + 0.5, to[2] + 0.5];
                cube_tinted(
                    mesh,
                    feet,
                    Quat::from_rotation_y(std::f32::consts::PI),
                    1.0,
                    saddle,
                    sky,
                    block,
                    inflated_from,
                    inflated_to,
                    uv,
                    pivot,
                    rotate_body,
                    [1.0; 3],
                    [64., 64.],
                    Some(dimensions),
                );
            }
        }
    }
}
