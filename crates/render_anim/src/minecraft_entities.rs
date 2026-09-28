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
use minecraft_terrain::walk_animation::WalkAnimations;
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
    walk: WalkAnimations,
    clock: f64,
    ticks: u64,
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
    pub(crate) fn new(stream: &TerrainStream) -> Self {
        let sim = ServerSim::new(stream.world_gen(), stream.states.clone(), "minecraft:overworld");
        Self {
            server: ServerHandle::spawn(sim),
            world: EntityWorld::default(),
            walk: WalkAnimations::default(),
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
                let w = &self.world;
                self.walk.tick(
                    w.creepers()
                        .iter()
                        .map(|e| (e.id, e.previous_position, e.creeper.body.position, false))
                        .chain(w.spiders().iter().map(|e| (e.id, e.previous_position, e.spider.body.position, false)))
                        .chain(w.endermen().iter().map(|e| (e.id, e.previous_position, e.enderman.body.position, false)))
                        .chain(w.witches().iter().map(|e| (e.id, e.previous_position, e.witch.body.position, false)))
                        .chain(w.iron_golems().iter().map(|e| (e.id, e.previous_position, e.golem.body.position, false)))
                        .chain(w.wolves().iter().map(|e| (e.id, e.previous_position, e.wolf.body.position, e.wolf.baby())))
                        .chain(
                            w.villagers()
                                .iter()
                                .map(|e| (e.id, e.previous_position, e.villager.body.position, e.villager.age.baby())),
                        ),
                );
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

    /// The mobs, drawn with the viewer's renderers into block-space meshes.
    pub(crate) fn append_mesh(
        &self,
        mesh: &mut ChunkMesh,
        scene: &HandcraftedScene,
        atlas: &Atlas,
        light: &SkyLight,
        forward: Vec3,
    ) {
        use minecraft_terrain::*;
        let w = &self.world;
        let partial = (self.clock / TICK_SECONDS).clamp(0.0, 1.0) as f32;
        cow_render::append_cows(mesh, w.cows().iter(), atlas, light, partial);
        sheep_render::append_sheep(mesh, w.sheep().iter(), atlas, light);
        pig_render::append_pigs(mesh, w.pigs().iter(), atlas, light);
        chicken_render::append_chickens(mesh, w.chickens().iter(), atlas, light, partial);
        bat_render::append_bats(mesh, w.bats().iter(), atlas, light, partial);
        zombie_render::append_zombies(mesh, w.zombies().iter(), atlas, light, partial);
        creeper_render::append_creepers(mesh, w.creepers().iter(), &self.walk, atlas, light, partial);
        spider_render::append_spiders(mesh, w.spiders().iter(), &self.walk, atlas, light, partial);
        skeleton_render::append_skeletons(mesh, w.skeletons().iter(), atlas, light, partial);
        villager_render::append_villagers(
            mesh,
            w.villagers().iter(),
            &self.walk,
            &|pos| Scene::block(scene, pos).and_then(|b| b.properties.get("facing").cloned()),
            atlas,
            light,
            partial,
        );
        let mut translucent = ChunkMesh::default();
        slime_render::append_slimes(mesh, &mut translucent, w.slimes().iter(), atlas, light, partial);
        let _ = enderman_render::append_endermen(mesh, w.endermen().iter(), &self.walk, atlas, light, partial, 0);
        witch_render::append_witches(mesh, w.witches().iter(), &self.walk, atlas, light, partial);
        let _ = golem_render::append_iron_golems(mesh, w.iron_golems().iter(), &self.walk, atlas, light, partial);
        wolf_render::append_wolves(mesh, w.wolves().iter(), &self.walk, atlas, light, partial, w.game_time());
        flame_render::append_flames(
            mesh,
            w.burning()
                .into_iter()
                .map(|(previous, now, width, height)| (previous.lerp(now, f64::from(partial)), width, height)),
            atlas,
            forward,
            light,
        );
        witch_render::append_potions(
            mesh,
            w.potions()
                .iter()
                .filter(|p| p.potion.alive)
                .map(|p| (p, p.previous_position.lerp(p.potion.position, f64::from(partial)))),
            atlas,
            forward,
            light,
        );
    }
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
