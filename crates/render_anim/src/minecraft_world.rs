//! The Minecraft map at run time: MinecraftOSS generates and streams a seeded
//! world around the local player, whose chunks become block collision and
//! whose section meshes go to the renderer. The world sits with its player
//! spawn at map origin, a block to `sim::voxel::BLOCK` map units.
use std::collections::HashMap;
use std::sync::{Arc, mpsc};

use bevy::prelude::*;
use minecraft_terrain::clouds::CloudMask;
use minecraft_terrain::day_cycle::{DayCycle, Skybox};
use minecraft_terrain::environment::{DimensionEnvironment, View};
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::{Atlas, SectionMesh, Vertex};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::HandcraftedScene;
use minecraft_terrain::sections::{CullCamera, SectionPos};
use minecraft_terrain::terrain::{Dimension, TerrainStream};
use minecraftoss_core::BlockStateId;
use minecraftoss_core::registries::{DataPaths, Registries};

const VIEW_DISTANCE: i32 = 8;
const TICK_SECONDS: f64 = 1.0 / 20.0;
/// Blocks on a side of the light volume MW2 models are lit from.
pub const LIGHT_VOLUME: i32 = 64;
/// Chunk sections fade in over this long, as the viewer's default option.
const FADE_MILLIS: u64 = 750;

/// What the renderer takes from the world each frame.
#[derive(Resource, Default)]
pub struct MinecraftWorldView {
    /// A Minecraft map is loaded: the stand-in map's world is not drawn.
    pub active: bool,
    /// Block point at map origin.
    pub origin: [f64; 3],
    pub atlas: Option<Arc<Atlas>>,
    pub uploads: Vec<(SectionPos, SectionMesh)>,
    pub removed: Vec<SectionPos>,
    pub visible: Vec<(SectionPos, f32)>,
    /// Bumped when the world is replaced, so stale sections are dropped.
    pub generation: u64,
    /// The environment uniform of MinecraftOSS for this frame, in block space.
    pub environment: [[f32; 4]; 16],
    /// Sun and the eight moon phases, 32 pixels each, side by side.
    pub celestial: Option<Arc<image::RgbaImage>>,
    pub clouds: Option<Arc<(Vec<Vertex>, Vec<u32>)>>,
    /// Sky and block light around the player: origin block, then
    /// `LIGHT_VOLUME` cubed pairs, x fastest then z then y.
    pub light_volume: Option<Arc<([i32; 3], Vec<u8>)>>,
    /// Sky and block light at the eye, for the view model.
    pub eye_light: [f32; 2],
    /// Break particles as section vertices and indices, rebuilt each frame.
    pub particles: (Vec<u8>, Vec<u32>),
    /// Destroy stage cubes over blocks being mined: position, strip uv.
    pub cracks: (Vec<[f32; 5]>, Vec<u32>),
    /// The ten destroy stages side by side.
    pub crack_texture: Option<Arc<image::RgbaImage>>,
    /// Mob models (cut out, back-face culled, translucent) and entity
    /// shadows, as `mesh::Vertex` bytes and indices.
    pub entity_meshes: [(Vec<u8>, Vec<u32>); 4],
}

struct Loaded {
    stream: TerrainStream,
    scene: HandcraftedScene,
    packs: PackStack,
    atlas: Arc<Atlas>,
    registries: Arc<Registries>,
    seed: i64,
    environment: DimensionEnvironment,
    celestial: Arc<image::RgbaImage>,
    cloud_mask: Option<CloudMask>,
    crack_texture: Arc<image::RgbaImage>,
}

#[derive(Default)]
struct Runtime {
    loading: Option<mpsc::Receiver<Result<Loaded, String>>>,
    world: Option<Loaded>,
    day: DayCycle,
    environment_accumulator: f64,
    environment_primed: bool,
    light: Option<SkyLight>,
    light_volume_at: Option<[i32; 3]>,
    light_volume_age: u32,
    cloud_center: Option<(i32, i32)>,
    mining: crate::minecraft_mining::Mining,
    inventory_ui: crate::minecraft_inventory::InventoryUi,
    entities: Option<crate::minecraft_entities::Entities>,
    /// Shape id of each block state already seen.
    shapes: HashMap<BlockStateId, u16>,
    /// Boxes of each shape id, to reuse an id for a repeated shape.
    shape_ids: HashMap<Vec<[u32; 6]>, u16>,
    was_alive: bool,
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<MinecraftWorldView>()
        .init_resource::<frame::MinecraftUi>()
        .init_resource::<frame::InventoryPuppet>()
        .insert_non_send(Runtime::default())
        .add_systems(
            Update,
            update
                .after(frame::PresentedPublished)
                .in_set(frame::ClientSet::Present),
        );
}

fn load(seed: i64) -> Result<Loaded, String> {
    let root = assets::minecraft_map::root().ok_or("MINECRAFTOSS_ROOT is not set")?;
    let paths = DataPaths::discover()?;
    let registries = Arc::new(Registries::load(&paths)?);
    let packs = PackStack::open(vec![root.join("resourcepacks/local/minecraft-26.3")])
        .map_err(|e| e.to_string())?;
    let stream = TerrainStream::for_dimension(
        registries.clone(),
        seed,
        VIEW_DISTANCE,
        Dimension::Overworld,
        None,
    )
    .map_err(|e| e.to_string())?;
    let build = minecraft_terrain::mesh::build(&HandcraftedScene::default(), &packs)
        .map_err(|e| e.to_string())?;
    let scene = HandcraftedScene::streamed(stream.states.clone());
    let environment =
        DimensionEnvironment::load(&registries, Dimension::Overworld.dimension_type())?;
    let celestial = Arc::new(celestial_image(&packs).map_err(|e| e.to_string())?);
    let cloud_mask = CloudMask::from_pack(&packs).ok();
    let crack_texture = Arc::new(crate::minecraft_mining::crack_strip(&packs).map_err(|e| e.to_string())?);
    Ok(Loaded {
        stream,
        scene,
        packs,
        atlas: build.atlas,
        registries,
        seed,
        environment,
        celestial,
        cloud_mask,
        crack_texture,
    })
}

/// The sky's sun and moon phases, laid out as MinecraftOSS lays them out.
fn celestial_image(packs: &PackStack) -> anyhow::Result<image::RgbaImage> {
    let mut celestial = image::RgbaImage::new(32 * 9, 32);
    let names = [
        "environment/celestial/sun",
        "environment/celestial/moon/full_moon",
        "environment/celestial/moon/waning_gibbous",
        "environment/celestial/moon/third_quarter",
        "environment/celestial/moon/waning_crescent",
        "environment/celestial/moon/new_moon",
        "environment/celestial/moon/waxing_crescent",
        "environment/celestial/moon/first_quarter",
        "environment/celestial/moon/waxing_gibbous",
    ];
    for (index, path) in names.iter().enumerate() {
        let id = minecraft_terrain::pack::ResourceId::parse(&format!("minecraft:{path}"))?;
        if let Some(bytes) = packs.texture(&id)? {
            let img =
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
            let tile =
                image::imageops::resize(&img, 32, 32, image::imageops::FilterType::Nearest);
            image::imageops::replace(&mut celestial, &tile, (index as i64) * 32, 0);
        }
    }
    Ok(celestial)
}

fn seed() -> i64 {
    if let Some(seed) = std::env::var("IW4L_MINECRAFT_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        return seed;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    (nanos as i64) ^ 0x5DEE_CE66_D1CE_4E5B
}

#[allow(clippy::too_many_arguments)]
fn update(
    time: Res<Time>,
    mut installed: MessageReader<frame::MatchInstalled>,
    mut torn_down: MessageReader<frame::MatchTornDown>,
    local: Res<net::LocalPresentClient>,
    presented: Res<net::PresentedSnapshot>,
    authority: Option<ResMut<net::AuthorityWorld>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    (mut ui, mut puppet, mut images): (
        ResMut<frame::MinecraftUi>,
        ResMut<frame::InventoryPuppet>,
        ResMut<Assets<Image>>,
    ),
    mut view: ResMut<MinecraftWorldView>,
    mut runtime: NonSendMut<Runtime>,
) {
    for _ in torn_down.read() {
        stop(&mut runtime, &mut view);
    }
    for match_ in installed.read() {
        stop(&mut runtime, &mut view);
        if assets::minecraft_map::is_minecraft(&match_.zone) {
            let seed = seed();
            diag::info!(World, "Minecraft world: seed {seed}");
            let (send, receive) = mpsc::channel();
            let _ = std::thread::Builder::new()
                .name("minecraft-world-load".into())
                .spawn(move || {
                    let _ = send.send(load(seed));
                });
            runtime.loading = Some(receive);
            view.active = true;
        }
    }
    let Some(mut authority) = authority else {
        return;
    };

    if let Some(receive) = &runtime.loading
        && let Ok(result) = receive.try_recv()
    {
        runtime.loading = None;
        match result {
            Ok(world) => {
                let (x, y, z) = world.stream.player_spawn;
                view.origin = [x, y, z];
                view.atlas = Some(world.atlas.clone());
                view.celestial = Some(world.celestial.clone());
                view.crack_texture = Some(world.crack_texture.clone());
                runtime.mining = Default::default();
                runtime.entities = Some(crate::minecraft_entities::Entities::new(&world.stream, world.seed));
                runtime.day = DayCycle::default();
                // Game ticks since sunrise to start at: 6000 noon, 13000
                // dusk, 18000 midnight.
                if let Some(ticks) = std::env::var("IW4L_MINECRAFT_TIME")
                    .ok()
                    .and_then(|t| t.trim().parse::<f64>().ok())
                {
                    runtime.day.set(ticks);
                }
                runtime.environment_accumulator = 0.0;
                runtime.environment_primed = false;
                runtime.light = Some(SkyLight::streamed());
                runtime.light_volume_at = None;
                runtime.cloud_center = None;
                view.generation += 1;
                sim::voxel::activate(
                    authority.0.content().clip_brushes(),
                    view.origin,
                    vec![Vec::new()],
                );
                diag::info!(
                    World,
                    "Minecraft world ready: seed {} spawn {:?}",
                    world.seed,
                    world.stream.player_spawn
                );
                runtime.world = Some(world);
                runtime.shapes.clear();
                runtime.shape_ids.clear();
                runtime.was_alive = false;
            }
            Err(error) => {
                diag::warn!(World, "Minecraft world failed to load: {error}");
                view.active = false;
            }
        }
    }

    let origin = view.origin;
    let Runtime {
        world,
        shapes,
        shape_ids,
        was_alive,
        day,
        environment_accumulator,
        environment_primed,
        light,
        light_volume_at,
        light_volume_age,
        cloud_center,
        mining,
        entities,
        inventory_ui,
        ..
    } = &mut *runtime;
    let Some(world) = world.as_mut() else {
        ui.active = false;
        ui.inventory_open = false;
        puppet.active = false;
        return;
    };
    let Some(ps) = presented.player(local.0) else {
        ui.active = false;
        puppet.active = false;
        return;
    };

    // Every spawn lands on the Minecraft spawn once its ground exists.
    let alive = ps.pm_type == 0;
    let spawn_chunk = ((origin[0].floor() as i32) >> 4, (origin[2].floor() as i32) >> 4);
    if alive && !*was_alive && world.scene.generated_chunk(spawn_chunk).is_some() {
        // Retried each frame until the authority has the player to move.
        if authority.0.teleport(local.0, [0.0, 0.0, 0.0]) {
            diag::info!(World, "Minecraft spawn: moved to the world spawn");
            *was_alive = true;
        }
    } else if !alive {
        *was_alive = false;
    }

    let feet = sim::voxel::to_block(origin, ps.origin);
    let block = (
        feet[0].floor() as i32,
        feet[1].floor() as i32,
        feet[2].floor() as i32,
    );
    let (loaded, forgotten) = world.stream.server_tick(block, &mut world.scene);
    for chunk in loaded {
        let blocks = &world.registries.blocks;
        let (min_y, height) = (chunk.min_y(), chunk.height());
        let mut ids = vec![0u16; (height * 256) as usize];
        for y in 0..height {
            for z in 0..16usize {
                for x in 0..16usize {
                    let state = chunk.block(x, min_y + y, z);
                    let id = *shapes.entry(state).or_insert_with(|| {
                        let boxes = blocks.collision_boxes(state);
                        if boxes.is_empty() {
                            return 0;
                        }
                        let key: Vec<[u32; 6]> =
                            boxes.iter().map(|b| b.map(|v| (v as f32).to_bits())).collect();
                        if let Some(&id) = shape_ids.get(&key) {
                            return id;
                        }
                        let boxes32 = boxes.iter().map(|b| b.map(|v| v as f32)).collect();
                        let id = sim::voxel::add_shapes(vec![boxes32]).unwrap_or(0);
                        shape_ids.insert(key, id);
                        id
                    });
                    ids[((y as usize * 16) + z) * 16 + x] = id;
                }
            }
        }
        if let Some(entities) = entities.as_mut() {
            entities.load_chunk(&chunk);
        }
        sim::voxel::set_chunk(
            chunk.pos.x,
            chunk.pos.z,
            sim::voxel::VoxelChunk {
                min_y,
                height,
                shapes: ids,
            },
        );
    }
    for pos in forgotten {
        sim::voxel::remove_chunk(pos.x, pos.z);
        if let Some(entities) = entities.as_mut() {
            entities.unload_chunk(pos);
        }
    }

    // Shots and explosions from the authoritative game: bullets that met a
    // mob hurt it, the rest mine.
    let (mob_shots, events): (Vec<_>, Vec<_>) = sim::voxel::take_events()
        .into_iter()
        .partition(|event| matches!(event, sim::voxel::VoxelEvent::MobShot { .. }));
    // Minecraft's yaw: 0 facing +Z (map -Y), turning towards -X.
    let yaw_rad = ps.viewangles[1].to_radians();
    let mc_yaw = (-yaw_rad.cos()).atan2(-yaw_rad.sin()).to_degrees();
    if let Some(entities) = entities.as_mut() {
        for shot in mob_shots {
            if let sim::voxel::VoxelEvent::MobShot { key, damage, from } = shot {
                entities.shoot(key, damage, from, mc_yaw);
            }
        }
    }
    let broken = mining.apply(
        events,
        &mut crate::minecraft_mining::WorldRefs {
            stream: &mut world.stream,
            scene: &mut world.scene,
            packs: &world.packs,
            atlas: &world.atlas,
            registries: &world.registries,
        },
        time.elapsed_secs_f64(),
    );
    if let Some(entities) = entities.as_mut() {
        let positions: Vec<_> = broken.iter().map(|(pos, ..)| *pos).collect();
        entities.broke(&world.scene, &positions);
        entities.drop_blocks(&broken);
    }

    let eye = sim::voxel::to_block(origin, [ps.origin[0], ps.origin[1], ps.origin[2] + ps.view_height_current]);
    let (pitch, yaw) = (ps.viewangles[0].to_radians(), ps.viewangles[1].to_radians());
    let map_forward = [pitch.cos() * yaw.cos(), pitch.cos() * yaw.sin(), -pitch.sin()];
    let forward = glam::Vec3::new(map_forward[0], map_forward[2], -map_forward[1]);
    let aspect = windows
        .single()
        .map(|w| w.width() / w.height().max(1.0))
        .unwrap_or(16.0 / 9.0);
    let camera = CullCamera {
        position: glam::DVec3::new(eye[0], eye[1], eye[2]),
        forward,
        fov_degrees: 90.0,
        aspect,
        yaw_degrees: (-forward.x).atan2(forward.z).to_degrees(),
        pitch_degrees: (-forward.y).asin().to_degrees(),
    };
    let update = world
        .stream
        .frame(&world.scene, &camera, FADE_MILLIS, &world.atlas, &world.packs);
    view.uploads.extend(update.uploads);
    view.removed.extend(update.removed);
    view.visible = update.visible;
    let Some(light) = light.as_mut() else {
        return;
    };
    for (chunk, column) in update.lights {
        light.set_chunk_column(chunk, column);
    }

    // The Overworld clock and the environment attributes of MinecraftOSS.
    let dt = time.delta_secs_f64();
    let partial = mining.tick(&world.scene, dt);
    view.particles = mining.particle_mesh(&world.atlas, forward, partial, light);

    // The mobs: a server tick when due, the blocks it changed, its hits on
    // the player, the mobs' boxes for bullets and their meshes.
    if let Some(entities) = entities.as_mut() {
        let player = crate::minecraft_entities::PlayerView {
            feet,
            alive,
            health: ps.health as f32,
            yaw: mc_yaw,
            pitch: ps.viewangles[0],
        };
        let bright_outside = world.environment.sky_light_level() > 11.0;
        let ticks_before = entities.client_ticks();
        let (changes, hits) = entities.tick(dt, day.ticks as i64, bright_outside, &player);
        let mob_ticks = (entities.client_ticks() - ticks_before) as u32;
        if !changes.is_empty() {
            let blocks = &world.registries.blocks;
            let mut positions = Vec::with_capacity(changes.len());
            for (pos, block) in changes {
                let state = block.as_ref().and_then(|b| world.stream.states.state_of(b));
                let shape = state.map_or(0, |state| {
                    *shapes.entry(state).or_insert_with(|| {
                        let boxes = blocks.collision_boxes(state);
                        if boxes.is_empty() {
                            return 0;
                        }
                        let key: Vec<[u32; 6]> = boxes.iter().map(|b| b.map(|v| (v as f32).to_bits())).collect();
                        if let Some(&id) = shape_ids.get(&key) {
                            return id;
                        }
                        let boxes32 = boxes.iter().map(|b| b.map(|v| v as f32)).collect();
                        let id = sim::voxel::add_shapes(vec![boxes32]).unwrap_or(0);
                        shape_ids.insert(key, id);
                        id
                    })
                });
                sim::voxel::set_block_shape(pos.0, pos.1, pos.2, shape);
                world.scene.set(pos, block);
                positions.push(pos);
            }
            world.stream.mark_edited(&world.scene, &positions);
        }
        for (amount, from) in hits {
            sim::voxel::push_player_damage(local.0.0, amount, from.map(|b| sim::voxel::to_map(origin, b)));
        }
        // The inventory: MW2 guns as items, the HUD's clicks, the hotbar's
        // gun, and what the HUD shows.
        let owned: Vec<u32> = ps
            .weapons
            .iter()
            .filter(|&&w| w > 0)
            .map(|&w| w as u32)
            .filter(|&w| authority.0.weapon_combat_row(w).is_some_and(|facts| facts.inventory_type == 0))
            .collect();
        ui.active = alive;
        if !alive {
            ui.inventory_open = false;
        }
        inventory_ui.sync_weapons(&mut entities.inventory, &owned);
        let mut selected = entities.selected;
        let thrown = inventory_ui.apply_input(&mut ui, &mut entities.inventory, &mut selected);
        let thrower = crate::minecraft_inventory::Thrower {
            eye: glam::DVec3::from_array(eye),
            yaw: mc_yaw,
            pitch: ps.viewangles[0],
        };
        crate::minecraft_inventory::throw(&mut entities.world_items, thrown, &thrower);
        ui.weapon_request = inventory_ui.weapon_request(&entities.inventory, &mut selected, ps.weapon as u32);
        entities.selected = selected;
        inventory_ui.publish(&mut ui, &entities.inventory, selected, &world.packs, &mut images);

        // The player's MW2 body stands in the inventory's character box: in
        // front of the camera where the box shows, facing it, turned and
        // aiming towards the mouse.
        puppet.active = false;
        if ui.inventory_open
            && alive
            && let (Some([cx, cy, box_h]), Ok(window)) = (ui.character_box, windows.single())
        {
            let (w, h) = (window.width().max(1.0), window.height().max(1.0));
            let (pitch, yaw) = (ps.viewangles[0].to_radians(), ps.viewangles[1].to_radians());
            let fwd = Vec3::new(pitch.cos() * yaw.cos(), pitch.cos() * yaw.sin(), -pitch.sin());
            let right = Vec3::new(yaw.sin(), -yaw.cos(), 0.0);
            let up = right.cross(fwd).normalize_or(Vec3::Z);
            // Hor+ MW2: 65 degrees across at 4:3, so about 51 up the screen.
            let tan_v = (51.0f32.to_radians() * 0.5).tan();
            let distance = 30.0;
            let ndc = [cx / w * 2.0 - 1.0, 1.0 - cy / h * 2.0];
            let scale = (box_h / h) * 2.0 * distance * tan_v / 76.0;
            let eye_map = Vec3::new(ps.origin[0], ps.origin[1], ps.origin[2] + ps.view_height_current);
            let centre = eye_map
                + fwd * distance
                + right * (ndc[0] * distance * tan_v * (w / h))
                + up * (ndc[1] * distance * tan_v);
            let feet = centre - up * (36.0 * scale);
            // Facing the camera, turned by the mouse as vanilla's
            // `InventoryScreen.renderEntityInInventoryFollowsMouse` turns its body.
            let turn = (ui.gaze[0] * 1.2).atan() * 0.7;
            let facing = -fwd * turn.cos() + right * turn.sin();
            let x_axis = (facing - up * facing.dot(up)).normalize_or(-fwd);
            let y_axis = up.cross(x_axis);
            puppet.root = Mat4::from_cols(
                (x_axis * scale).extend(0.0),
                (y_axis * scale).extend(0.0),
                (up * scale).extend(0.0),
                feet.extend(1.0),
            );
            puppet.pitch = (ui.gaze[1] * 1.2).atan().to_degrees() * 0.6;
            puppet.client = local.0.0;
            puppet.active = true;
        }

        sim::voxel::set_mob_boxes(entities.boxes());
        entities.tick_scene(&world.scene, mob_ticks);
        let sky_darken = (15.0 - world.environment.sky_light_level()).clamp(0.0, 15.0) as u8;
        let meshes = entities.meshes(
            &world.scene,
            &world.packs,
            &world.atlas,
            light,
            forward,
            glam::DVec3::from_array(eye),
            sky_darken,
        );
        let raw = |mesh: &minecraft_terrain::mesh::ChunkMesh| {
            (bytemuck::cast_slice::<_, u8>(&mesh.vertices).to_vec(), mesh.indices.clone())
        };
        view.entity_meshes = [
            raw(&meshes.models),
            raw(&meshes.culled),
            raw(&meshes.translucent),
            raw(&meshes.shadows),
        ];
        let mesh = meshes.items;
        let (bytes, indices) = &mut view.particles;
        let base = (bytes.len() / std::mem::size_of::<minecraft_terrain::mesh::SectionVertex>()) as u32;
        let vertices: Vec<minecraft_terrain::mesh::SectionVertex> =
            mesh.vertices.iter().map(minecraft_terrain::mesh::SectionVertex::from_vertex).collect();
        bytes.extend_from_slice(bytemuck::cast_slice(&vertices));
        indices.extend(mesh.indices.iter().map(|i| i + base));
    }
    view.cracks = mining.crack_mesh();
    day.advance(dt);
    let eye_block = (eye[0].floor() as i32, eye[1].floor() as i32, eye[2].floor() as i32);
    view.eye_light = [
        f32::from(light.get(eye_block)),
        f32::from(light.get_block(eye_block)),
    ];
    world
        .environment
        .update_rain_fog(0.0, light.get(eye_block), false, (dt * 20.0) as f32);
    *environment_accumulator += dt;
    if !*environment_primed || *environment_accumulator >= TICK_SECONDS {
        *environment_accumulator = (*environment_accumulator % TICK_SECONDS).min(TICK_SECONDS);
        let scene = &world.scene;
        world.environment.tick(
            day.ticks.floor() as i64,
            0.0,
            0.0,
            eye,
            |x, y, z| scene.noise_biome((x, y, z)).map_or(0, |id| id.0),
            !*environment_primed,
        );
        *environment_primed = true;
    }
    let partial_tick = (*environment_accumulator / TICK_SECONDS).clamp(0.0, 1.0) as f32;
    let sky = world.environment.sky_state(&View {
        partial_tick,
        forward,
        camera_y: eye[1] as f32,
        render_distance: VIEW_DISTANCE as u32,
        rain_level: 0.0,
        thunder_level: 0.0,
    });
    let render_distance = VIEW_DISTANCE as f32 * 16.0;
    let right = forward.cross(glam::Vec3::Y).normalize_or(glam::Vec3::X);
    let up = right.cross(forward).normalize_or(glam::Vec3::Y);
    let put = |v: glam::Vec3| [v.x, v.y, v.z, 0.0];
    let game_time = day.ticks;
    view.environment = [
        put(forward),
        put(right),
        put(up),
        [eye[0] as f32, eye[1] as f32, eye[2] as f32, 0.0],
        put(sky.sky),
        [sky.fog.x, sky.fog.y, sky.fog.z, render_distance.min(sky.sky_fog_end)],
        [
            sky.sky_light_color.x,
            sky.sky_light_color.y,
            sky.sky_light_color.z,
            sky.sky_light_factor,
        ],
        sky.sunset,
        [sky.sun_direction.x, sky.sun_direction.y, sky.sun_direction.z, sky.rain_brightness],
        [sky.moon_direction.x, sky.moon_direction.y, sky.moon_direction.z, sky.rain_brightness],
        // The brightness option at its default.
        [sky.cloud.x, sky.cloud.y, sky.cloud.z, 0.5],
        [aspect, 0.0, sky.star_brightness, sky.star_angle],
        [sky.moon_phase as f32, (game_time as f32) * 0.03, 96.0, 160.0],
        [
            sky.fog_start,
            sky.fog_end,
            render_distance - (render_distance / 10.0).clamp(4.0, 64.0),
            render_distance,
        ],
        [
            sky.ambient.x,
            sky.ambient.y,
            sky.ambient.z,
            match sky.skybox {
                Skybox::Overworld => 0.0,
                Skybox::End => 1.0,
                _ => 2.0,
            },
        ],
        [
            sky.block_light_tint.x,
            sky.block_light_tint.y,
            sky.block_light_tint.z,
            sky.block_factor,
        ],
    ];

    // Clouds, rebuilt when the camera crosses a cloud cell.
    if let Some(mask) = &world.cloud_mask {
        let center = mask.center(eye[0] as f32, eye[2] as f32, game_time);
        if *cloud_center != Some(center) {
            *cloud_center = Some(center);
            let mesh = mask.build(center, eye[1] as f32);
            view.clouds = Some(Arc::new((mesh.vertices, mesh.indices)));
        }
    }

    // The light MW2 models stand in, around the player.
    *light_volume_age += 1;
    let half = LIGHT_VOLUME / 2;
    let corner = [block.0 - half, block.1 - half, block.2 - half];
    let moved =
        light_volume_at.is_none_or(|at| (0..3).any(|k| (at[k] - corner[k]).abs() >= 4));
    if moved || *light_volume_age >= 20 {
        *light_volume_age = 0;
        *light_volume_at = Some(corner);
        let n = LIGHT_VOLUME;
        let index = |x: i32, y: i32, z: i32| (((y * n + z) * n + x) * 2) as usize;
        let mut raw = vec![0u8; (n * n * n * 2) as usize];
        for y in 0..n {
            for z in 0..n {
                for x in 0..n {
                    let pos = (corner[0] + x, corner[1] + y, corner[2] + z);
                    let at = index(x, y, z);
                    raw[at] = light.get(pos);
                    raw[at + 1] = light.get_block(pos);
                }
            }
        }
        // Unlit cells (inside blocks) take their brightest neighbour, so a
        // model beside a block is not darkened by filtering into it. Levels
        // become unorm bytes.
        let mut data = vec![0u8; raw.len()];
        for y in 0..n {
            for z in 0..n {
                for x in 0..n {
                    let at = index(x, y, z);
                    let (mut sky, mut block) = (raw[at], raw[at + 1]);
                    if sky == 0 && block == 0 {
                        for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
                            let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                            if (0..n).contains(&nx) && (0..n).contains(&ny) && (0..n).contains(&nz) {
                                let near = index(nx, ny, nz);
                                sky = sky.max(raw[near]);
                                block = block.max(raw[near + 1]);
                            }
                        }
                    }
                    data[at] = sky.min(15) * 17;
                    data[at + 1] = block.min(15) * 17;
                }
            }
        }
        view.light_volume = Some(Arc::new((corner, data)));
    }
}

fn stop(runtime: &mut Runtime, view: &mut MinecraftWorldView) {
    if runtime.world.take().is_some() || runtime.loading.take().is_some() || view.active {
        sim::voxel::deactivate();
        view.active = false;
        view.atlas = None;
        view.uploads.clear();
        view.removed.clear();
        view.visible.clear();
        view.celestial = None;
        view.crack_texture = None;
        view.particles = Default::default();
        view.entity_meshes = Default::default();
        view.cracks = Default::default();
        view.clouds = None;
        view.light_volume = None;
        view.generation += 1;
    }
}
