//! The first-person hand on the Minecraft map when no gun is selected: the
//! held item, or vanilla's empty right arm, placed as MinecraftOSS's viewer
//! places them (26.3 `applyItemArmTransform` with the item's
//! `firstperson_righthand` display, `renderPlayerArm`), swinging as vanilla
//! swings them. Vertices are in view space: x right, y up, z back.
use glam::{Mat4, Vec3};
use minecraft_terrain::mesh::{Atlas, ChunkMesh, Vertex};
use minecraft_terrain::pack::ResourceId;

/// Ticks of a swing (`LivingEntity.getCurrentSwingDuration`).
pub(crate) const SWING_TICKS: f32 = 6.0;

/// The item's view-space pose (`applyItemArmTransform` then its display).
pub(crate) fn item_pose(display: Mat4, swing: f32, inverse_height: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(0.56, -0.52 - 0.6 * inverse_height, -0.72)) * item_swing_transform(swing) * display
}

fn item_swing_transform(swing: f32) -> Mat4 {
    let root = swing.sqrt();
    let x = -0.4 * (root * std::f32::consts::PI).sin();
    let y = 0.2 * (root * std::f32::consts::TAU).sin();
    let z = -0.2 * (swing * std::f32::consts::PI).sin();
    let y_rotation = (swing * swing * std::f32::consts::PI).sin();
    let xz_rotation = (root * std::f32::consts::PI).sin();
    Mat4::from_translation(Vec3::new(x, y, z))
        * Mat4::from_rotation_y((45.0 - 20.0 * y_rotation).to_radians())
        * Mat4::from_rotation_z((-20.0 * xz_rotation).to_radians())
        * Mat4::from_rotation_x((-80.0 * xz_rotation).to_radians())
        * Mat4::from_rotation_y((-45.0f32).to_radians())
}

/// Vanilla's empty right arm (`renderPlayerArm`, the wide model), skinned
/// from the atlas's default player skin, lit at `sky` and `block`.
pub(crate) fn empty_hand_mesh(atlas: &Atlas, swing: f32, inverse_height: f32, sky: f32, block: f32) -> ChunkMesh {
    let mut mesh = ChunkMesh::default();
    let Ok(skin) = ResourceId::parse("minecraft:entity/player/wide/steve") else {
        return mesh;
    };
    if !atlas.contains(&skin) {
        return mesh;
    }
    let [s0, t0, s1, t1] = atlas.entity_region(&skin);
    let root = swing.sqrt();
    let xs = -0.3 * (root * std::f32::consts::PI).sin();
    let ys = 0.4 * (root * std::f32::consts::TAU).sin();
    let zs = -0.4 * (swing * std::f32::consts::PI).sin();
    let transform = Mat4::from_translation(Vec3::new(xs + 0.64000005, ys - 0.6 - 0.6 * inverse_height, zs - 0.71999997))
        * Mat4::from_rotation_y(45.0f32.to_radians())
        * Mat4::from_rotation_y(((root * std::f32::consts::PI).sin() * 70.0).to_radians())
        * Mat4::from_rotation_z((-(swing * swing * std::f32::consts::PI).sin() * 20.0).to_radians())
        * Mat4::from_translation(Vec3::new(-1.0, 3.6, 3.5))
        * Mat4::from_rotation_z(120.0f32.to_radians())
        * Mat4::from_rotation_x(200.0f32.to_radians())
        * Mat4::from_rotation_y((-135.0f32).to_radians())
        * Mat4::from_translation(Vec3::new(5.6, 0.0, 0.0))
        * Mat4::from_translation(Vec3::new(-5.0 / 16.0, 2.0 / 16.0, 0.0))
        * Mat4::from_rotation_z(0.1);
    // The arm and its sleeve layer, as the viewer draws them.
    for (inflation, v_offset) in [(0.0f32, 16.0f32), (0.25, 32.0)] {
        let [x, y, z] = [(-3.0 - inflation) / 16.0, (-2.0 - inflation) / 16.0, (-2.0 - inflation) / 16.0];
        let [xx, yy, zz] = [(1.0 + inflation) / 16.0, (10.0 + inflation) / 16.0, (2.0 + inflation) / 16.0];
        let faces = [
            ([[xx, yy, z], [xx, y, z], [x, y, z], [x, yy, z]], [42.0, v_offset + 4.0, 46.0, v_offset + 16.0]),
            ([[x, yy, zz], [x, y, zz], [xx, y, zz], [xx, yy, zz]], [50.0, v_offset + 4.0, 54.0, v_offset + 16.0]),
            ([[x, yy, z], [x, y, z], [x, y, zz], [x, yy, zz]], [40.0, v_offset + 4.0, 44.0, v_offset + 16.0]),
            ([[xx, yy, zz], [xx, y, zz], [xx, y, z], [xx, yy, z]], [48.0, v_offset + 4.0, 52.0, v_offset + 16.0]),
            ([[x, yy, z], [x, yy, zz], [xx, yy, zz], [xx, yy, z]], [44.0, v_offset, 48.0, v_offset + 4.0]),
            ([[x, y, zz], [x, y, z], [xx, y, z], [xx, y, zz]], [48.0, v_offset, 52.0, v_offset + 4.0]),
        ];
        for (points, [u0, v0, u1, v1]) in faces {
            let start = mesh.vertices.len() as u32;
            for (point, [u, v]) in points.into_iter().zip([[u0, v0], [u0, v1], [u1, v1], [u1, v0]]) {
                // The 64x64 skin's pixel, in its atlas slot.
                mesh.vertices.push(Vertex {
                    position: transform.transform_point3(Vec3::from_array(point)).to_array(),
                    uv: [s0 + (s1 - s0) * (u / 64.0), t0 + (t1 - t0) * (v / 64.0)],
                    color: [1.0; 4],
                    sky_light: sky,
                    block_light: block,
                });
            }
            mesh.indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
            mesh.faces += 1;
        }
    }
    mesh
}
