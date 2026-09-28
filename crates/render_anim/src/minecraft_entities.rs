//! The Minecraft world's mobs, as MinecraftOSS runs them: its integrated
//! server (`engine/viewer/src/server.rs`) ticks the level at 20 Hz on its own
//! thread with natural spawning and the entity world, every passive and
//! hostile mob's AI and pathfinding. Here it is fed the streamed chunks and
//! the player, and hands back the tracked mobs, which are drawn with the
//! viewer's mob renderers. MW2 bullets hurt the mobs with their real damage
//! (a hundred MW2 health to Minecraft's twenty), and mobs hurt the player
//! the other way round.
use glam::{DVec3, Vec3};
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::{Atlas, ChunkMesh};
use minecraft_terrain::scene::{Block, HandcraftedScene, Scene};
use minecraft_terrain::server::{PlayerEdit, ServerHandle, ServerSim, TickInput};
use minecraft_terrain::terrain::TerrainStream;
use minecraft_terrain::client_mobs::{ClientMobs, server_mobs};
use minecraft_terrain::mesh::ItemVisuals;
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::poof_particles::PoofParticles;
use minecraft_terrain::portal_particles::PortalParticles;
use minecraftoss_entities::tempt::PlayerCandidate;
use minecraftoss_entities::world::{EntityWorld, MobHit, PlayerHitKind};

const TICK_SECONDS: f64 = 1.0 / 20.0;
/// Minecraft health per MW2 health.
const HEALTH_SCALE: f32 = 20.0 / 100.0;
/// The player's id in the entity world.
const PLAYER: u64 = 0;
/// `Player.getEyeHeight` standing.
const EYE_HEIGHT: f32 = 1.62;

pub(crate) struct Entities {
    server: ServerHandle,
    /// The mobs the player tracks, as of the last server tick.
    world: EntityWorld,
    /// The client's copies of them: tracked, interpolated, animated.
    client: ClientMobs,
    poof: PoofParticles,
    portal: PortalParticles,
    items: ItemVisuals,
    clock: f64,
    ticks: u64,
}

/// The mobs drawn this frame: entity models (cut out, back-face culled,
/// translucent) and their shadows, in `mesh::Vertex`s; and what goes with
/// the particles (held items, puffs, flames, potions).
#[derive(Default)]
pub(crate) struct MobMeshes {
    pub models: ChunkMesh,
    pub culled: ChunkMesh,
    pub translucent: ChunkMesh,
    pub shadows: ChunkMesh,
    pub items: ChunkMesh,
}

/// The player as the mobs see it this frame.
pub(crate) struct PlayerView {
    pub feet: [f64; 3],
    pub alive: bool,
    /// MW2 health, 0..100.
    pub health: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Entities {
    pub(crate) fn new(stream: &TerrainStream, seed: i64) -> Self {
        let sim = ServerSim::new(stream.world_gen(), stream.states.clone(), "minecraft:overworld");
        let mut server = ServerHandle::spawn(sim);
        // Mob loot, from the game's data JAR when MinecraftOSS has one.
        if let Some(jar) = data_jar() {
            server.load_loot(jar, seed as u64);
        }
        Self {
            server,
            world: EntityWorld::default(),
            client: ClientMobs::default(),
            poof: PoofParticles::default(),
            portal: PortalParticles::default(),
            items: ItemVisuals::default(),
            clock: 0.0,
            ticks: 0,
        }
    }

    pub(crate) fn load_chunk(&mut self, chunk: &std::sync::Arc<minecraftoss_core::Chunk>) {
        self.server.load_chunk(chunk);
    }

    pub(crate) fn unload_chunk(&mut self, pos: minecraftoss_core::ChunkPos) {
        self.server.unload_chunk(pos);
    }

    /// Client ticks run so far.
    pub(crate) fn client_ticks(&self) -> u64 {
        self.ticks
    }

    /// Blocks the player's weapons broke, for the level the mobs walk in.
    pub(crate) fn broke(&mut self, scene: &HandcraftedScene, positions: &[(i32, i32, i32)]) {
        for &pos in positions {
            self.server.player_edit(scene, pos, PlayerEdit::Break);
        }
    }

    /// A bullet on a mob: an attack with the bullet's damage from where it
    /// was fired.
    pub(crate) fn shoot(&mut self, key: u64, damage: f32, from: [f64; 3], yaw: f32) {
        let Some(hit) = decode(key) else {
            return;
        };
        let attack = minecraftoss_entities::world::PlayerAttack {
            player_id: PLAYER,
            position: DVec3::from_array(from),
            yaw,
            attack_damage: f64::from(damage * HEALTH_SCALE),
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        };
        self.server
            .mob_action(hit, Some(attack), &minecraftoss_player::inventory::Inventory::default(), 0, false);
    }

    /// Sends a server tick when one is due and takes what came back: block
    /// changes to show, and mob hits on the player in MW2 damage.
    pub(crate) fn tick(
        &mut self,
        dt: f64,
        day_ticks: i64,
        bright_outside: bool,
        player: &PlayerView,
    ) -> (Vec<((i32, i32, i32), Option<Block>)>, Vec<(i32, Option<[f64; 3]>)>) {
        self.clock += dt;
        if self.clock >= TICK_SECONDS {
            self.clock = (self.clock - TICK_SECONDS).min(TICK_SECONDS);
            self.ticks += 1;
            // Packets first, then the client level's entity ticks.
            self.client.tick();
            for enderman in self.world.endermen() {
                if let Some(mob) = self.client.get(enderman.id) {
                    self.portal.emit_enderman(mob.position, 0.6, 2.9);
                }
            }
            self.portal.tick();
            if self.ticks % 600 == 0 {
                diag::info!(World, "Minecraft mobs: {} tracked", self.boxes().len());
            }
            let feet = DVec3::from_array(player.feet);
            let center = ((feet.x.floor() as i32) >> 4, (feet.z.floor() as i32) >> 4);
            let candidate = PlayerCandidate {
                id: PLAYER,
                position: feet,
                eye_height: EYE_HEIGHT,
                main_hand_cow_food: false,
                offhand_cow_food: false,
                main_hand_pig_food: false,
                offhand_pig_food: false,
                main_hand_chicken_food: false,
                offhand_chicken_food: false,
                main_hand_carrot_on_a_stick: false,
                offhand_carrot_on_a_stick: false,
                main_hand_wolf_interest: false,
                offhand_wolf_interest: false,
                main_hand_horse_tempt: false,
                offhand_horse_tempt: false,
                alive: player.alive,
                spectator: !player.alive,
                attackable: player.alive,
            };
            self.server.tick(TickInput {
                day_ticks,
                players: if player.alive { vec![player.feet] } else { Vec::new() },
                difficulty: 2,
                simulation_center: center,
                simulation_distance: 8,
                pickup: None,
                mob_players: vec![candidate],
                mob_views: vec![(
                    PLAYER,
                    minecraftoss_entities::enderman::PlayerView {
                        head_yaw: player.yaw,
                        pitch: player.pitch,
                        disguised: false,
                    },
                )],
                mob_vitals: vec![(
                    PLAYER,
                    minecraftoss_entities::monster_ai::PlayerVitals {
                        health: player.health * HEALTH_SCALE,
                        ..Default::default()
                    },
                )],
                bright_outside,
                tracking: (player.feet, 160.0),
                player_hurts: Vec::new(),
                spawn_mobs: true,
            });
        }
        let mut changes = Vec::new();
        let mut hits = Vec::new();
        for output in self.server.poll() {
            changes.extend(output.changes);
            if let Some(mobs) = output.mobs {
                self.world = *mobs;
                for (feet, width, height) in self.client.receive(server_mobs(&self.world)) {
                    self.poof.spawn(feet, width, height);
                }
            }
            for hit in output.player_hits.into_iter().filter(|h| h.player_id == PLAYER) {
                let from = match hit.kind {
                    PlayerHitKind::Melee { attacker, .. } => Some(attacker.to_array()),
                    _ => None,
                };
                hits.push(((hit.damage / HEALTH_SCALE).round() as i32, from));
            }
        }
        (changes, hits)
    }

    /// Every living mob's key and box in blocks, for bullets.
    pub(crate) fn boxes(&self) -> Vec<(u64, [f64; 6])> {
        let w = &self.world;
        let mut out = Vec::new();
        let mut add = |hit: MobHit, body: &minecraftoss_entities::movement::Body| {
            let p = body.position;
            let half = f64::from(body.width) * 0.5;
            out.push((
                encode(hit),
                [p.x - half, p.y, p.z - half, p.x + half, p.y + f64::from(body.height), p.z + half],
            ));
        };
        for e in w.bats().iter().filter(|e| e.bat.health > 0.0) {
            add(MobHit::Bat(e.id), &e.bat.body);
        }
        for e in w.zombies().iter().filter(|e| e.zombie.health > 0.0) {
            add(MobHit::Zombie(e.id), &e.zombie.body);
        }
        for e in w.skeletons().iter().filter(|e| e.skeleton.health > 0.0) {
            add(MobHit::Skeleton(e.id), &e.skeleton.body);
        }
        for e in w.creepers().iter().filter(|e| e.creeper.health > 0.0 && !e.creeper.exploded) {
            add(MobHit::Creeper(e.id), &e.creeper.body);
        }
        for e in w.spiders().iter().filter(|e| e.spider.health > 0.0) {
            add(MobHit::Spider(e.id), &e.spider.body);
        }
        for e in w.slimes().iter().filter(|e| e.slime.health > 0.0) {
            add(MobHit::Slime(e.id), &e.slime.body);
        }
        for e in w.endermen().iter().filter(|e| e.enderman.health > 0.0) {
            add(MobHit::Enderman(e.id), &e.enderman.body);
        }
        for e in w.witches().iter().filter(|e| e.witch.health > 0.0) {
            add(MobHit::Witch(e.id), &e.witch.body);
        }
        for e in w.iron_golems().iter().filter(|e| e.golem.health > 0.0) {
            add(MobHit::IronGolem(e.id), &e.golem.body);
        }
        for e in w.wolves().iter().filter(|e| e.wolf.health > 0.0) {
            add(MobHit::Wolf(e.id), &e.wolf.body);
        }
        for e in w.villagers().iter().filter(|e| e.villager.health > 0.0) {
            add(MobHit::Villager(e.id), &e.villager.body);
        }
        for e in w.cows().iter().filter(|e| e.cow.health > 0.0) {
            let hit = if e.mooshroom.is_some() { MobHit::Mooshroom(e.id) } else { MobHit::Cow(e.id) };
            add(hit, &e.cow.body);
        }
        for e in w.sheep().iter().filter(|e| e.health > 0.0) {
            add(MobHit::Sheep(e.id), &e.body);
        }
        for e in w.pigs().iter().filter(|e| e.pig.health > 0.0) {
            add(MobHit::Pig(e.id), &e.pig.body);
        }
        for e in w.chickens().iter().filter(|e| e.chicken.health > 0.0) {
            add(MobHit::Chicken(e.id), &e.chicken.body);
        }
        out
    }

    /// Steps the client-side puffs, which settle on the scene's blocks.
    pub(crate) fn tick_scene(&mut self, scene: &HandcraftedScene, ticks: u32) {
        for _ in 0..ticks {
            self.poof.tick(scene);
        }
    }

    /// The mobs this frame, drawn with the viewer's renderers.
    pub(crate) fn meshes(
        &mut self,
        scene: &HandcraftedScene,
        packs: &PackStack,
        atlas: &Atlas,
        light: &SkyLight,
        forward: Vec3,
        camera: DVec3,
        sky_darken: u8,
    ) -> MobMeshes {
        use minecraft_terrain::*;
        // Mobs appear on the client once their trackers start.
        self.client.spawn_missing(&server_mobs(&self.world));
        let w = &self.world;
        let poses = &self.client;
        let partial = (self.clock / TICK_SECONDS).clamp(0.0, 1.0) as f32;
        let mut out = MobMeshes::default();
        cow_render::append_cows(&mut out.models, w.cows().iter(), poses, atlas, light, partial);
        sheep_render::append_sheep(&mut out.models, w.sheep().iter(), poses, atlas, light, partial);
        pig_render::append_pigs(&mut out.models, w.pigs().iter(), poses, atlas, light, partial);
        chicken_render::append_chickens(&mut out.models, w.chickens().iter(), poses, atlas, light, partial);
        bat_render::append_bats(&mut out.culled, w.bats().iter(), poses, atlas, light, partial);
        let zombie_items = zombie_render::append_zombies(&mut out.models, w.zombies().iter(), poses, atlas, light, partial);
        creeper_render::append_creepers(&mut out.models, w.creepers().iter(), poses, atlas, light, partial);
        spider_render::append_spiders(&mut out.models, w.spiders().iter(), poses, atlas, light, partial);
        let skeleton_items =
            skeleton_render::append_skeletons(&mut out.models, w.skeletons().iter(), poses, atlas, light, partial);
        let villager_items = villager_render::append_villagers(
            &mut out.models,
            w.villagers().iter(),
            poses,
            &|pos| Scene::block(scene, pos).and_then(|b| b.properties.get("facing").cloned()),
            atlas,
            light,
            partial,
        );
        horse_render::append_horses(&mut out.models, &mut out.translucent, w.cows().iter(), poses, atlas, light, partial);
        slime_render::append_slimes(&mut out.models, &mut out.translucent, w.slimes().iter(), poses, atlas, light, partial);
        let carried = enderman_render::append_endermen(
            &mut out.models,
            w.endermen().iter(),
            poses,
            atlas,
            light,
            partial,
            self.ticks.rotate_left(32) ^ (partial.to_bits() as u64),
        );
        let witch_items = witch_render::append_witches(&mut out.models, w.witches().iter(), poses, atlas, light, partial);
        let poppies = golem_render::append_iron_golems(&mut out.models, w.iron_golems().iter(), poses, atlas, light, partial);
        wolf_render::append_wolves(&mut out.models, w.wolves().iter(), poses, atlas, light, partial, w.game_time());
        flame_render::append_flames(
            &mut out.items,
            w.burning()
                .into_iter()
                .map(|(previous, now, width, height)| (previous.lerp(now, f64::from(partial)), width, height)),
            atlas,
            forward,
            light,
        );
        witch_render::append_potions(
            &mut out.items,
            w.potions()
                .iter()
                .filter(|p| p.potion.alive)
                .map(|p| (p, p.previous_position.lerp(p.potion.position, f64::from(partial)))),
            atlas,
            forward,
            light,
        );
        self.poof.append_mesh(&mut out.items, atlas, forward, partial, light);
        self.portal.append_mesh(&mut out.items, atlas, forward, partial, light);
        let held: Vec<_> =
            skeleton_items.into_iter().chain(zombie_items).chain(villager_items).chain(witch_items).collect();
        let _ = self.items.append_held_items(&mut out.items, &held, packs, atlas, light);
        let carried: Vec<_> = carried.into_iter().chain(poppies).collect();
        let _ = self.items.append_posed_blocks(&mut out.items, &carried, packs, atlas, light);
        // Their shadows, at `getMaxLocalRawBrightness`.
        let casters = client_mobs::shadow_casters(w, poses, camera, partial);
        let raw = |pos: (i32, i32, i32)| light.get(pos).saturating_sub(sky_darken).max(light.get_block(pos));
        if let Ok(shadows) = mesh::entity_shadows(&casters, scene, atlas, &raw, 0.0) {
            out.shadows = shadows;
        }
        out
    }
}

/// The game's data JAR the MinecraftOSS harness downloaded (loot tables,
/// recipes), as the viewer finds it.
pub(crate) fn data_jar() -> Option<std::path::PathBuf> {
    let root = assets::minecraft_map::root()?.join("harness/.gradle/loom-cache/minecraftMaven/net/minecraft");
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("minecraft-common-") {
            continue;
        }
        for jar in std::fs::read_dir(entry.path().join("26.3")).ok()?.flatten() {
            if jar.path().extension().is_some_and(|ext| ext == "jar") {
                return Some(jar.path());
            }
        }
    }
    None
}

/// A mob hit as one number: the kind in the top byte, the id below.
fn encode(hit: MobHit) -> u64 {
    let kind: u64 = match hit {
        MobHit::Bat(_) => 0,
        MobHit::Zombie(_) => 1,
        MobHit::Skeleton(_) => 2,
        MobHit::Creeper(_) => 3,
        MobHit::Spider(_) => 4,
        MobHit::Slime(_) => 5,
        MobHit::Enderman(_) => 6,
        MobHit::Witch(_) => 7,
        MobHit::IronGolem(_) => 8,
        MobHit::Wolf(_) => 9,
        MobHit::Villager(_) => 10,
        MobHit::Cow(_) => 11,
        MobHit::Mooshroom(_) => 12,
        MobHit::Sheep(_) => 13,
        MobHit::Pig(_) => 14,
        MobHit::Chicken(_) => 15,
    };
    (kind << 56) | (hit.id() & ((1 << 56) - 1))
}

fn decode(key: u64) -> Option<MobHit> {
    let id = key & ((1 << 56) - 1);
    Some(match key >> 56 {
        0 => MobHit::Bat(id),
        1 => MobHit::Zombie(id),
        2 => MobHit::Skeleton(id),
        3 => MobHit::Creeper(id),
        4 => MobHit::Spider(id),
        5 => MobHit::Slime(id),
        6 => MobHit::Enderman(id),
        7 => MobHit::Witch(id),
        8 => MobHit::IronGolem(id),
        9 => MobHit::Wolf(id),
        10 => MobHit::Villager(id),
        11 => MobHit::Cow(id),
        12 => MobHit::Mooshroom(id),
        13 => MobHit::Sheep(id),
        14 => MobHit::Pig(id),
        15 => MobHit::Chicken(id),
        _ => return None,
    })
}
