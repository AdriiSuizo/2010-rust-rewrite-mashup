//! Pack-backed pinned 26.3 spider: `SpiderModel.createSpiderBodyLayer` (a
//! head, a small and a large body segment, and eight legs splayed out
//! sideways), its `setupAnim` (the head's turn and pitch; the legs sweep
//! back and forth and lift in four phases with the walk), and
//! `SpiderEyesLayer`'s eyes: the same model with `spider_eyes`, drawn at
//! full brightness where vanilla adds it unlit. The death flip is not drawn.
use crate::{
    cow_render::cube_scaled,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
    walk_animation::WalkAnimations,
};
use glam::{Quat, Vec3};
use minecraftoss_entities::world::SpiderEntity;
use std::f32::consts::PI;

type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3]);

const HEAD: Part = ([-4., -4., -8.], [4., 4., 0.], [32., 4.], [0., 15., -3.]);
const BODY_0: Part = ([-3., -3., -3.], [3., 3., 3.], [0., 0.], [0., 15., 0.]);
const BODY_1: Part = ([-5., -4., -6.], [5., 4., 6.], [0., 12.], [0., 15., 9.]);

/// A leg's pivot, resting yaw and roll, and which of the four walk phases
/// it follows (hind, middle hind, middle front, front). Right legs reach
/// out along -X; left legs are their mirror.
const LEGS: [([f32; 3], f32, f32, usize); 8] = [
    ([-4., 15., 2.], 0.785_398_2, -0.785_398_2, 0),
    ([4., 15., 2.], -0.785_398_2, 0.785_398_2, 0),
    ([-4., 15., 1.], 0.392_699_1, -0.581_194_64, 1),
    ([4., 15., 1.], -0.392_699_1, 0.581_194_64, 1),
    ([-4., 15., 0.], -0.392_699_1, -0.581_194_64, 2),
    ([4., 15., 0.], 0.392_699_1, 0.581_194_64, 2),
    ([-4., 15., -1.], -0.785_398_2, -0.785_398_2, 3),
    ([4., 15., -1.], 0.785_398_2, 0.785_398_2, 3),
];

/// `SpiderModel.setupAnim`'s leg sweep (yaw) and lift (roll) for each of
/// the four phases at a walk position and speed.
pub fn leg_motion(position: f32, speed: f32) -> [(f32, f32); 4] {
    let p = position * 0.6662;
    [0.0, PI, PI / 2.0, 4.712_389].map(|phase| {
        let swing = -((p * 2.0 + phase).cos() * 0.4) * speed;
        let step = ((p + phase).sin() * 0.4).abs() * speed;
        (swing, step)
    })
}

pub fn append_spiders<'a>(
    mesh: &mut ChunkMesh,
    spiders: impl IntoIterator<Item = &'a SpiderEntity>,
    walks: &WalkAnimations,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let skin = atlas.entity_region(&ResourceId::parse("minecraft:entity/spider/spider").unwrap());
    let eyes_id = ResourceId::parse("minecraft:entity/spider/spider_eyes").unwrap();
    let eyes = atlas.contains(&eyes_id).then(|| atlas.entity_region(&eyes_id));
    let partial = partial.clamp(0.0, 1.0);
    for entity in spiders {
        let spider = &entity.spider;
        if spider.health <= 0.0 {
            continue;
        }
        let feet = entity.previous_position.lerp(spider.body.position, f64::from(partial));
        let sample = (feet.x.floor() as i32, (feet.y + f64::from(spider.eye_height())).floor() as i32, feet.z.floor() as i32);
        let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
        let body_yaw = entity.ai.body_rotation.body_yaw;
        let rotation = Quat::from_rotation_y(PI - body_yaw.to_radians());
        let look = &entity.ai.state.look_control;
        let head = Quat::from_euler(glam::EulerRot::ZYX, 0.0, (look.head_yaw - body_yaw).to_radians(), look.pitch.to_radians());
        let walk = walks.get(entity.id);
        let motion = leg_motion(walk.position(partial), walk.speed(partial));
        let mut parts: Vec<(Part, Quat, bool)> = vec![(HEAD, head, false), (BODY_0, Quat::IDENTITY, false), (BODY_1, Quat::IDENTITY, false)];
        for (pivot, yaw, roll, phase) in LEGS {
            let left = pivot[0] > 0.0;
            let (swing, step) = motion[phase];
            let (yaw, roll) = if left { (yaw - swing, roll - step) } else { (yaw + swing, roll + step) };
            let (from, to) = if left { ([-1., -1., -1.], [15., 1., 1.]) } else { ([-15., -1., -1.], [1., 1., 1.]) };
            parts.push(((from, to, [18., 0.], pivot), Quat::from_euler(glam::EulerRot::ZYX, roll, yaw, 0.0), left));
        }
        for &((from, to, uv, pivot), pose, mirror) in &parts {
            cube_scaled(mesh, feet, rotation, Vec3::ONE, skin, sky, block, from, to, uv, pivot, pose, [1.0; 3], [64., 32.], None, mirror);
        }
        if let Some(eyes) = eyes {
            let ((from, to, uv, pivot), pose, _) = parts[0];
            cube_scaled(mesh, feet, rotation, Vec3::ONE, eyes, 15.0, 15.0, from, to, uv, pivot, pose, [1.0; 3], [64., 32.], None, false);
        }
    }
}
