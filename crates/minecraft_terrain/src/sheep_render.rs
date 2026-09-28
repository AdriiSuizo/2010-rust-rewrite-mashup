//! Source-informed pinned 26.3 sheep model layers with pack-resolved textures.
//! The authored NoAI sheep uses a fixed pose until its active animation lands.
use crate::{
    cow_render::cube_tinted,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::Quat;
use minecraftoss_entities::world::SheepEntity;

const DYE_DIFFUSE: [u32; 16] = [
    16383998, 16351261, 13061821, 3847130, 16701501, 8439583, 15961002, 4673362, 10329495, 1481884,
    8991416, 3949738, 8606770, 6192150, 11546150, 1908001,
];

fn wool_tint(id: usize) -> [f32; 3] {
    // ColorLerper.Type.SHEEP uses a special white and floors 0.75 times
    // DyeColor.getTextureDiffuseColor's channels for every other dye.
    let rgb = if id == 0 { 0xe6e6e6 } else { DYE_DIFFUSE[id] };
    let channel = |shift| {
        let value = ((rgb >> shift) & 255_u32) as f32;
        if id == 0 {
            value / 255.0
        } else {
            (value * 0.75).floor() / 255.0
        }
    };
    [channel(16), channel(8), channel(0)]
}

type BoxPart = ([f32; 3], [f32; 3], [f32; 2], [f32; 3], bool);

const ADULT_BODY: [BoxPart; 6] = [
    (
        [-3., -4., -6.],
        [3., 2., 2.],
        [0., 0.],
        [0., 6., -8.],
        false,
    ),
    (
        [-4., -10., -7.],
        [4., 6., -1.],
        [28., 8.],
        [0., 5., 2.],
        true,
    ),
    (
        [-2., 0., -2.],
        [2., 12., 2.],
        [0., 16.],
        [-3., 12., 7.],
        false,
    ),
    (
        [-2., 0., -2.],
        [2., 12., 2.],
        [0., 16.],
        [3., 12., 7.],
        false,
    ),
    (
        [-2., 0., -2.],
        [2., 12., 2.],
        [0., 16.],
        [-3., 12., -5.],
        false,
    ),
    (
        [-2., 0., -2.],
        [2., 12., 2.],
        [0., 16.],
        [3., 12., -5.],
        false,
    ),
];

const ADULT_FUR: [BoxPart; 6] = [
    (
        [-3.6, -4.6, -4.6],
        [3.6, 2.6, 2.6],
        [0., 0.],
        [0., 6., -8.],
        false,
    ),
    (
        [-5.75, -11.75, -8.75],
        [5.75, 7.75, 0.75],
        [28., 8.],
        [0., 5., 2.],
        true,
    ),
    (
        [-2.5, -0.5, -2.5],
        [2.5, 6.5, 2.5],
        [0., 16.],
        [-3., 12., 7.],
        false,
    ),
    (
        [-2.5, -0.5, -2.5],
        [2.5, 6.5, 2.5],
        [0., 16.],
        [3., 12., 7.],
        false,
    ),
    (
        [-2.5, -0.5, -2.5],
        [2.5, 6.5, 2.5],
        [0., 16.],
        [-3., 12., -5.],
        false,
    ),
    (
        [-2.5, -0.5, -2.5],
        [2.5, 6.5, 2.5],
        [0., 16.],
        [3., 12., -5.],
        false,
    ),
];

const BABY_BODY: [BoxPart; 6] = [
    (
        [-3., -2., -4.5],
        [3., 2., 4.5],
        [0., 10.],
        [0., 17., 0.5],
        false,
    ),
    (
        [-2.5, -4.5, -3.5],
        [2.5, 0.5, 1.5],
        [0., 0.],
        [0., 15.5, -2.5],
        false,
    ),
    (
        [-1., 0., -1.],
        [1., 5., 1.],
        [0., 23.],
        [-2., 19., 3.],
        false,
    ),
    (
        [-1., 0., -1.],
        [1., 5., 1.],
        [24., 12.],
        [2., 19., 3.],
        false,
    ),
    (
        [-1., 0., -1.],
        [1., 5., 1.],
        [8., 23.],
        [-2., 19., -2.],
        false,
    ),
    (
        [-1., 0., -1.],
        [1., 5., 1.],
        [24., 5.],
        [2., 19., -2.],
        false,
    ),
];

pub fn append_sheep<'a>(mesh: &mut ChunkMesh, sheep: impl IntoIterator<Item = &'a SheepEntity>, atlas: &Atlas, light: &SkyLight) {
    for entity in sheep {
        if entity.health <= 0.0 {
            continue;
        }
        let baby = entity.sheep.age.baby();
        let feet = entity.body.position;
        let rotation = Quat::from_rotation_y(std::f32::consts::PI);
        let pos = (
            feet.x.floor() as i32,
            (feet.y + 0.7).floor() as i32,
            feet.z.floor() as i32,
        );
        let sky = light.get(pos) as f32;
        let block = light.get_block(pos) as f32;
        let base = if baby {
            "minecraft:entity/sheep/sheep_baby"
        } else {
            "minecraft:entity/sheep/sheep"
        };
        append_layer(
            mesh,
            atlas,
            base,
            feet,
            rotation,
            sky,
            block,
            if baby { &BABY_BODY } else { &ADULT_BODY },
            [1.0; 3],
            false,
        );
        let color = (entity.sheep.wool.data() & 15) as usize;
        let tint = wool_tint(color);
        if !baby && color != 0 {
            append_layer(
                mesh,
                atlas,
                "minecraft:entity/sheep/sheep_wool_undercoat",
                feet,
                rotation,
                sky,
                block,
                &ADULT_FUR,
                tint,
                true,
            );
        }
        if !entity.sheep.wool.sheared() {
            let texture = if baby {
                "minecraft:entity/sheep/sheep_wool_baby"
            } else {
                "minecraft:entity/sheep/sheep_wool"
            };
            append_layer(
                mesh,
                atlas,
                texture,
                feet,
                rotation,
                sky,
                block,
                if baby { &BABY_BODY } else { &ADULT_FUR },
                tint,
                !baby,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_layer(
    mesh: &mut ChunkMesh,
    atlas: &Atlas,
    texture: &str,
    feet: glam::DVec3,
    rotation: Quat,
    sky: f32,
    block: f32,
    parts: &[BoxPart],
    tint: [f32; 3],
    fur: bool,
) {
    let region = atlas.entity_region(&ResourceId::parse(texture).unwrap());
    for (index, &(from, to, uv, pivot, rotate_body)) in parts.iter().enumerate() {
        let uv_dimensions = if fur {
            Some(if index == 0 {
                [6.0, 6.0, 6.0]
            } else if index == 1 {
                [8.0, 16.0, 6.0]
            } else {
                [4.0, 6.0, 4.0]
            })
        } else {
            None
        };
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
            tint,
            [64.0, 32.0],
            uv_dimensions,
        );
    }
}
