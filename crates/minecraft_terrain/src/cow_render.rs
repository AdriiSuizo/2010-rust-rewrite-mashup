//! Pack-backed cow body mesh. Geometry/UVs are from pinned 26.3 CowModel;
//! the image is always resolved through the active resource-pack atlas.
use crate::{
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh, Vertex},
    pack::ResourceId,
};
use glam::{DVec3, Quat, Vec3};
use minecraftoss_entities::{cow::CowVariant, mooshroom::MushroomVariant, world::CowEntity};

pub fn texture_id(variant: CowVariant, baby: bool) -> ResourceId {
    let kind = match variant {
        CowVariant::Temperate => "temperate",
        CowVariant::Warm => "warm",
        CowVariant::Cold => "cold",
    };
    ResourceId::parse(&format!(
        "minecraft:entity/cow/cow_{kind}{}",
        if baby { "_baby" } else { "" }
    ))
    .unwrap()
}

pub fn append_cows<'a>(
    mesh: &mut ChunkMesh,
    cows: impl IntoIterator<Item = &'a CowEntity>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    for entity in cows {
        if entity.cow.health <= 0.0 {
            continue;
        }
        let cow = &entity.cow;
        let feet = entity
            .previous_position
            .lerp(cow.body.position, partial.clamp(0.0, 1.0) as f64);
        let yaw_delta = (cow.yaw - entity.previous_yaw + 180.0).rem_euclid(360.0) - 180.0;
        let yaw = entity.previous_yaw + yaw_delta * partial.clamp(0.0, 1.0);
        let rotation = Quat::from_rotation_y(std::f32::consts::PI - yaw.to_radians());
        let sprite = if let Some(state) = &entity.mooshroom {
            let kind = match state.variant {
                MushroomVariant::Red => "red",
                MushroomVariant::Brown => "brown",
            };
            ResourceId::parse(&format!(
                "minecraft:entity/cow/mooshroom_{kind}{}",
                if cow.age.baby() { "_baby" } else { "" }
            ))
            .unwrap()
        } else {
            texture_id(cow.variant, cow.age.baby())
        };
        let region = atlas.entity_region(&sprite);
        let pos = (
            feet.x.floor() as i32,
            (feet.y + 0.7).floor() as i32,
            feet.z.floor() as i32,
        );
        let sky = light.get(pos) as f32;
        let block = light.get_block(pos) as f32;
        let scale = 1.0;
        if cow.age.baby() {
            // LayerDefinitions maps all three COW_*_BABY layers to
            // BabyCowModel.createBodyLayer; only the texture varies.
            let baby_boxes = [
                (
                    [-3., -4.569, -4.8333],
                    [3., 1.431, 0.1667],
                    [0., 18.],
                    [0., 13.569, -5.1667],
                ),
                (
                    [3., -5.569, -3.8333],
                    [4., -3.569, -2.8333],
                    [8., 29.],
                    [0., 13.569, -5.1667],
                ),
                (
                    [-4., -5.569, -3.8333],
                    [-3., -3.569, -2.8333],
                    [4., 29.],
                    [0., 13.569, -5.1667],
                ),
                (
                    [-2., -1.569, -5.8333],
                    [2., 1.431, -4.8333],
                    [12., 29.],
                    [0., 13.569, -5.1667],
                ),
                ([-7., -7., -1.], [1., -1., 11.], [0., 0.], [3., 19., -5.]),
                (
                    [-1.5, 0., -1.5],
                    [1.5, 6., 1.5],
                    [22., 18.],
                    [-2.5, 18., -3.5],
                ),
                (
                    [-1.5, 0., -1.5],
                    [1.5, 6., 1.5],
                    [34., 18.],
                    [2.5, 18., -3.5],
                ),
                (
                    [-1.5, 0., -1.5],
                    [1.5, 6., 1.5],
                    [22., 27.],
                    [-2.5, 18., 3.5],
                ),
                (
                    [-1.5, 0., -1.5],
                    [1.5, 6., 1.5],
                    [34., 27.],
                    [2.5, 18., 3.5],
                ),
            ];
            for (from, to, uv, pivot) in baby_boxes {
                cube(
                    mesh, feet, rotation, scale, region, sky, block, from, to, uv, pivot, false,
                );
            }
            continue;
        }
        // Native model coordinates are downward-positive with its feet at Y=24.
        // The adult layer is CowModel.createBaseCowModel (26.3).
        let boxes: [([f32; 3], [f32; 3], [f32; 2], [f32; 3], bool); 7] = [
            (
                [-4., -4., -6.],
                [4., 4., 0.],
                [0., 0.],
                [0., 4., -8.],
                false,
            ),
            (
                [-3., 1., -7.],
                [3., 4., -6.],
                [1., 33.],
                [0., 4., -8.],
                false,
            ),
            (
                [-6., -10., -7.],
                [6., 8., 3.],
                [18., 4.],
                [0., 5., 2.],
                true,
            ),
            ([-2., 2., -8.], [2., 8., -7.], [52., 0.], [0., 5., 2.], true),
            (
                [-2., 0., -2.],
                [2., 12., 2.],
                [0., 16.],
                [-4., 12., 7.],
                false,
            ),
            (
                [-2., 0., -2.],
                [2., 12., 2.],
                [0., 16.],
                [4., 12., 7.],
                false,
            ),
            (
                [-2., 0., -2.],
                [2., 12., 2.],
                [0., 16.],
                [-4., 12., -5.],
                false,
            ),
        ];
        for (from, to, uv, pivot, rotate_body) in boxes {
            cube(
                mesh,
                feet,
                rotation,
                scale,
                region,
                sky,
                block,
                from,
                to,
                uv,
                pivot,
                rotate_body,
            );
        }
        cube(
            mesh,
            feet,
            rotation,
            scale,
            region,
            sky,
            block,
            [-2., 0., -2.],
            [2., 12., 2.],
            [0., 16.],
            [4., 12., -5.],
            false,
        );
        let head = [0., 4., -8.];
        let horns: &[([f32; 3], [f32; 3], [f32; 2])] = match cow.variant {
            CowVariant::Temperate => &[
                ([-5., -5., -5.], [-4., -2., -4.], [22., 0.]),
                ([4., -5., -5.], [5., -2., -4.], [22., 0.]),
            ],
            CowVariant::Warm => &[
                ([-8., -3., -5.], [-4., -1., -3.], [27., 0.]),
                ([-8., -5., -5.], [-6., -3., -3.], [39., 0.]),
                ([4., -3., -5.], [8., -1., -3.], [27., 0.]),
                ([6., -5., -5.], [8., -3., -3.], [39., 0.]),
            ],
            CowVariant::Cold => &[],
        };
        for &(from, to, uv) in horns {
            cube(
                mesh, feet, rotation, scale, region, sky, block, from, to, uv, head, false,
            );
        }
        if cow.variant == CowVariant::Cold {
            // ColdCowModel adds a half-pixel wool shell and two curved horns.
            cube_tinted(
                mesh,
                feet,
                rotation,
                scale,
                region,
                sky,
                block,
                [-6.5, -10.5, -7.5],
                [6.5, 8.5, 3.5],
                [20., 32.],
                [0., 5., 2.],
                true,
                [1.; 3],
                [64.; 2],
                Some([12., 18., 10.]),
            );
            for &(from, to, uv, pivot) in &[
                (
                    [-1.5, -4.5, -0.5],
                    [0.5, 1.5, 1.5],
                    [0., 40.],
                    [-4.5, 1.5, -11.5],
                ),
                (
                    [-1.5, -3., -0.5],
                    [0.5, 3., 1.5],
                    [0., 32.],
                    [5.5, 1.5, -13.],
                ),
            ] {
                cube(
                    mesh, feet, rotation, scale, region, sky, block, from, to, uv, pivot, true,
                );
            }
        }
        if let Some(state) = &entity.mooshroom {
            // MushroomCowMushroomLayer renders three small cross-model
            // mushrooms from the variant's block state.
            let block_texture = ResourceId::parse(match state.variant {
                MushroomVariant::Red => "minecraft:block/red_mushroom",
                MushroomVariant::Brown => "minecraft:block/brown_mushroom",
            })
            .unwrap();
            let mushroom_region = atlas.region(&block_texture);
            for pivot in [[-4., 3., 4.], [4., 3., 4.], [0., 2., -9.]] {
                mushroom_cross(
                    mesh,
                    feet,
                    rotation,
                    1.0,
                    mushroom_region,
                    sky,
                    block,
                    pivot,
                );
            }
        }
    }
}

/// A model point (pixels, y down) in the entity's frame before its body
/// turn: `LivingEntityRenderer`'s `scale(-1, -1, 1)`, the renderer's own
/// scale, then `translate(0, -1.501, 0)`.
fn model_to_entity(p: Vec3, scale: Vec3) -> Vec3 {
    Vec3::new(-p.x, 24.016 - p.y, p.z) * scale / 16.0
}

#[allow(clippy::too_many_arguments)]
fn mushroom_cross(
    mesh: &mut ChunkMesh,
    feet: DVec3,
    rotation: Quat,
    scale: f32,
    region: [f32; 4],
    sky: f32,
    block: f32,
    pivot: [f32; 3],
) {
    let half = 4.0;
    for (dx, dz) in [(1.0, 1.0), (1.0, -1.0)] {
        let corners = [
            [-half * dx, half, -half * dz],
            [half * dx, half, half * dz],
            [half * dx, -half, half * dz],
            [-half * dx, -half, -half * dz],
        ];
        let start = mesh.vertices.len() as u32;
        for (corner, uv) in corners.into_iter().zip([
            [region[0], region[3]],
            [region[2], region[3]],
            [region[2], region[1]],
            [region[0], region[1]],
        ]) {
            let p = Vec3::from_array(corner) + Vec3::from_array(pivot);
            let local = model_to_entity(p, Vec3::splat(scale));
            let world = feet.as_vec3() + rotation * local;
            mesh.vertices.push(Vertex {
                position: world.to_array(),
                uv,
                color: [1.0, 1.0, 1.0, 1.0],
                sky_light: sky,
                block_light: block,
            });
        }
        mesh.indices.extend_from_slice(&[
            start,
            start + 1,
            start + 2,
            start,
            start + 2,
            start + 3,
            start + 2,
            start + 1,
            start,
            start + 3,
            start + 2,
            start,
        ]);
        mesh.faces += 2;
    }
}

#[allow(clippy::too_many_arguments)]
fn cube(
    mesh: &mut ChunkMesh,
    feet: DVec3,
    rotation: Quat,
    scale: f32,
    region: [f32; 4],
    sky: f32,
    block: f32,
    from: [f32; 3],
    to: [f32; 3],
    uv: [f32; 2],
    pivot: [f32; 3],
    rotate_body: bool,
) {
    cube_tinted(
        mesh,
        feet,
        rotation,
        scale,
        region,
        sky,
        block,
        from,
        to,
        uv,
        pivot,
        rotate_body,
        [1.0; 3],
        [64.0; 2],
        None,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cube_tinted(
    mesh: &mut ChunkMesh,
    feet: DVec3,
    rotation: Quat,
    scale: f32,
    region: [f32; 4],
    sky: f32,
    block: f32,
    from: [f32; 3],
    to: [f32; 3],
    uv: [f32; 2],
    pivot: [f32; 3],
    rotate_body: bool,
    tint: [f32; 3],
    texture_size: [f32; 2],
    uv_dimensions: Option<[f32; 3]>,
) {
    cube_tinted_pose(
        mesh,
        feet,
        rotation,
        scale,
        region,
        sky,
        block,
        from,
        to,
        uv,
        pivot,
        if rotate_body {
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
        } else {
            Quat::IDENTITY
        },
        tint,
        texture_size,
        uv_dimensions,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cube_tinted_pose(
    mesh: &mut ChunkMesh,
    feet: DVec3,
    rotation: Quat,
    scale: f32,
    region: [f32; 4],
    sky: f32,
    block: f32,
    from: [f32; 3],
    to: [f32; 3],
    uv: [f32; 2],
    pivot: [f32; 3],
    part_rotation: Quat,
    tint: [f32; 3],
    texture_size: [f32; 2],
    uv_dimensions: Option<[f32; 3]>,
) {
    cube_tinted_pose_mirror(
        mesh,
        feet,
        rotation,
        scale,
        region,
        sky,
        block,
        from,
        to,
        uv,
        pivot,
        part_rotation,
        tint,
        texture_size,
        uv_dimensions,
        false,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cube_tinted_pose_mirror(
    mesh: &mut ChunkMesh,
    feet: DVec3,
    rotation: Quat,
    scale: f32,
    region: [f32; 4],
    sky: f32,
    block: f32,
    from: [f32; 3],
    to: [f32; 3],
    uv: [f32; 2],
    pivot: [f32; 3],
    part_rotation: Quat,
    tint: [f32; 3],
    texture_size: [f32; 2],
    uv_dimensions: Option<[f32; 3]>,
    mirror: bool,
) {
    cube_scaled(mesh, feet, rotation, Vec3::splat(scale), region, sky, block, from, to, uv, pivot, part_rotation, tint, texture_size, uv_dimensions, mirror);
}

/// A model cuboid with a per-axis model scale about the feet (a renderer's
/// `scale` step, as `CreeperRenderer` swells).
#[allow(clippy::too_many_arguments)]
pub(crate) fn cube_scaled(
    mesh: &mut ChunkMesh,
    feet: DVec3,
    rotation: Quat,
    scale: Vec3,
    region: [f32; 4],
    sky: f32,
    block: f32,
    from: [f32; 3],
    to: [f32; 3],
    uv: [f32; 2],
    pivot: [f32; 3],
    part_rotation: Quat,
    tint: [f32; 3],
    texture_size: [f32; 2],
    uv_dimensions: Option<[f32; 3]>,
    mirror: bool,
) {
    let [x0, y0, z0] = from;
    let [x1, y1, z1] = to;
    let (x_lo, x_hi) = if mirror { (x1, x0) } else { (x0, x1) };
    let (w, h, d) = (x1 - x0, y1 - y0, z1 - z0);
    let [uw, uh, ud] = uv_dimensions.unwrap_or([w, h, d]);
    let (u, v) = (uv[0], uv[1]);
    // Pinned 26.3 ModelPart.Cube polygon order: DOWN, UP, WEST, NORTH,
    // EAST, SOUTH. Its Polygon maps vertex UVs as (u1,v0), (u0,v0),
    // (u0,v1), (u1,v1).
    let u0 = u;
    let u1 = u + ud;
    let u2 = u1 + uw;
    let u22 = u2 + uw;
    let u3 = u2 + ud;
    let u4 = u3 + uw;
    let v0 = v;
    let v1 = v + ud;
    let v2 = v1 + uh;
    let faces = [
        (
            [
                [x_hi, y0, z1],
                [x_lo, y0, z1],
                [x_lo, y0, z0],
                [x_hi, y0, z0],
            ],
            [u1, v0, u2, v1],
            0.55,
        ),
        (
            [
                [x_hi, y1, z0],
                [x_lo, y1, z0],
                [x_lo, y1, z1],
                [x_hi, y1, z1],
            ],
            [u2, v1, u22, v0],
            1.0,
        ),
        (
            [
                [x_lo, y0, z0],
                [x_lo, y0, z1],
                [x_lo, y1, z1],
                [x_lo, y1, z0],
            ],
            [u0, v1, u1, v2],
            0.70,
        ),
        (
            [
                [x_hi, y0, z0],
                [x_lo, y0, z0],
                [x_lo, y1, z0],
                [x_hi, y1, z0],
            ],
            [u1, v1, u2, v2],
            0.92,
        ),
        (
            [
                [x_hi, y0, z1],
                [x_hi, y0, z0],
                [x_hi, y1, z0],
                [x_hi, y1, z1],
            ],
            [u2, v1, u3, v2],
            0.76,
        ),
        (
            [
                [x_lo, y0, z1],
                [x_hi, y0, z1],
                [x_hi, y1, z1],
                [x_lo, y1, z1],
            ],
            [u3, v1, u4, v2],
            0.82,
        ),
    ];
    for (corners, tex, shade) in faces {
        let start = mesh.vertices.len() as u32;
        let mut vertices = std::array::from_fn::<_, 4, _>(|i| {
            (
                corners[i],
                [
                    [tex[2], tex[1]],
                    [tex[0], tex[1]],
                    [tex[0], tex[3]],
                    [tex[2], tex[3]],
                ][i],
            )
        });
        // ModelPart.Polygon reverses the already UV-mapped vertices after
        // Cube mirrors its X coordinates. This preserves outward normals and
        // swaps each face's skin orientation exactly as the pinned client.
        if mirror {
            vertices.reverse();
        }
        for (corner, coord) in vertices {
            let p = part_rotation * Vec3::from_array(corner) + Vec3::from_array(pivot);
            let local = model_to_entity(p, scale);
            let world = feet.as_vec3() + rotation * local;
            mesh.vertices.push(Vertex {
                position: world.to_array(),
                uv: [
                    region[0] + (region[2] - region[0]) * coord[0] / texture_size[0],
                    region[1] + (region[3] - region[1]) * coord[1] / texture_size[1],
                ],
                color: [shade * tint[0], shade * tint[1], shade * tint[2], 1.0],
                sky_light: sky,
                block_light: block,
            });
        }
        mesh.indices
            .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
}
