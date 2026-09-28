//! Minecraft's natural spawning, played by MW2 bots. MinecraftOSS's per-tick
//! spawner (`NaturalSpawner.spawnForChunk` around the players) runs on the
//! streamed world as a server would run it: its biome spawn lists, light and
//! moon rules, pack sizes and mob caps. Each monster it spawns becomes a spot
//! for the next MW2 bot waiting to spawn; the living bots are the monsters it
//! counts against its caps.
use std::sync::Arc;

use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::scene::{HandcraftedScene, Scene};
use minecraft_terrain::terrain::TerrainStream;
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::{LegacyRandom, RandomSource};
use minecraftoss_core::{BiomeId, BlockPos, BlockStateId, ChunkPos};
use minecraftoss_world::distance::PlayerChunkCounter;
use minecraftoss_world::natural_spawner::spawning::{MOON_BRIGHTNESS_PER_PHASE, MobCategory, SpawnContext};
use minecraftoss_world::natural_spawner::tick::{CensusMob, SpawnPlayer, TickInput, census_of};
use minecraftoss_world::natural_spawner::{CreatureSpawns, SpawnLevel};

/// Bots that play the world's monsters.
pub(crate) const MOB_BOTS: u32 = 10;
const TICK_SECONDS: f64 = 1.0 / 20.0;
/// `DistanceManager`'s natural spawn distance, in chunks.
const SPAWN_DISTANCE: i32 = 8;
/// Normal difficulty.
const DIFFICULTY: i32 = 2;

pub(crate) struct Mobs {
    spawns: Arc<CreatureSpawns>,
    counter: PlayerChunkCounter,
    registered: Option<ChunkPos>,
    random: LegacyRandom,
    entity_random: LegacyRandom,
    clock: f64,
    seed: i64,
}

/// What one spawning tick reads.
pub(crate) struct MobWorld<'a> {
    pub stream: &'a TerrainStream,
    pub scene: &'a HandcraftedScene,
    pub light: &'a SkyLight,
    /// `getSkyDarken`: 15 less the sky light level of the time of day.
    pub sky_darken: i32,
    pub overworld_time: i64,
    pub day: i32,
    /// The world's block point at map origin.
    pub origin: [f64; 3],
}

impl Mobs {
    pub(crate) fn new(spawns: Arc<CreatureSpawns>, seed: i64) -> Self {
        Self {
            spawns,
            counter: PlayerChunkCounter::new(SPAWN_DISTANCE),
            registered: None,
            random: LegacyRandom::new(seed ^ 0x5DEE_CE66),
            entity_random: LegacyRandom::new(seed.rotate_left(17)),
            clock: 0.0,
            seed,
        }
    }

    /// Runs the spawner at 20 ticks a second around the player's `feet`
    /// (blocks), with `mobs` (blocks) the monsters alive. Each monster it
    /// spawns is offered as a bot spawn spot.
    pub(crate) fn tick(&mut self, dt: f64, world: &MobWorld<'_>, feet: [f64; 3], mobs: &[[f64; 3]]) {
        self.clock += dt;
        if self.clock < TICK_SECONDS {
            return;
        }
        self.clock = (self.clock - TICK_SECONDS).min(TICK_SECONDS);

        let chunk = ChunkPos::new((feet[0].floor() as i32) >> 4, (feet[2].floor() as i32) >> 4);
        if self.registered != Some(chunk) {
            if let Some(old) = self.registered {
                self.counter.remove_player(old);
            }
            self.counter.add_player(chunk);
            self.registered = Some(chunk);
        }
        let players = [SpawnPlayer { pos: feet, spectator: false }];
        let feet_list = [feet];
        // Monsters, as zombies: the category is what the caps count.
        let census: Vec<CensusMob> = mobs
            .iter()
            .map(|&pos| CensusMob {
                kind: "minecraft:zombie".to_owned(),
                pos,
                bb: [pos[0] - 0.3, pos[1], pos[2] - 0.3, pos[0] + 0.3, pos[1] + 1.95, pos[2] + 0.3],
                persistent: false,
                riding: false,
            })
            .collect();
        let obstacles: Vec<[f64; 6]> = census.iter().map(|m| m.bb).chain(std::iter::once([
            feet[0] - 0.3,
            feet[1],
            feet[2] - 0.3,
            feet[0] + 0.3,
            feet[1] + 1.8,
            feet[2] + 0.3,
        ]))
        .collect();
        // Only lit, generated chunks spawn: an unlit chunk has no darkness to
        // spawn in yet.
        let loaded = |c: ChunkPos| {
            world.scene.generated_chunk((c.x, c.z)).is_some() && world.light.chunk_column((c.x, c.z)).is_some()
        };
        let slime = |_: BiomeId| 0.0f32;
        let mut context = SpawnContext {
            players: &feet_list,
            respawn: None,
            difficulty: DIFFICULTY,
            overworld_time: world.overworld_time,
            moon_brightness: MOON_BRIGHTNESS_PER_PHASE[world.day.rem_euclid(8) as usize],
            raining: false,
            thundering: false,
            seed: self.seed,
            surface_slime_chance: &slime,
            halloween: false,
            can_spawn_in_chunk: &loaded,
            obstacles,
            chickens: Vec::new(),
        };
        let mut input = TickInput {
            counter: &mut self.counter,
            players: &players,
            census: &census,
            spawn_mobs: true,
            spawn_enemies: true,
            spawn_persistent: false,
            loaded: &loaded,
            ticking: &loaded,
            entity_ticking_range: &loaded,
        };
        let mut level = Level {
            stream: world.stream,
            scene: world.scene,
            light: world.light,
            sky_darken: world.sky_darken,
            random: &mut self.random,
            entity_random: &mut self.entity_random,
            spawned: Vec::new(),
        };
        let spawns = self.spawns.clone();
        spawns.tick_spawning(&mut level, &mut context, &mut input);
        for tag in &level.spawned {
            for mob in census_of(tag) {
                if MobCategory::of_type(&mob.kind) != Some(MobCategory::Monster) {
                    continue;
                }
                let yaw = self.random.next_f32() * 360.0;
                sim::voxel::offer_mob_spawn(sim::voxel::to_map(world.origin, mob.pos), yaw);
            }
        }
    }
}

/// The streamed world as the natural spawner reads it.
struct Level<'a> {
    stream: &'a TerrainStream,
    scene: &'a HandcraftedScene,
    light: &'a SkyLight,
    sky_darken: i32,
    random: &'a mut LegacyRandom,
    entity_random: &'a mut LegacyRandom,
    spawned: Vec<Tag>,
}

impl SpawnLevel for Level<'_> {
    fn block(&self, pos: BlockPos) -> BlockStateId {
        Scene::block(self.scene, (pos.x, pos.y, pos.z))
            .and_then(|block| self.stream.states.state_of(block))
            .unwrap_or(BlockStateId::AIR)
    }

    fn height(&self, kind: HeightmapKind, x: i32, z: i32) -> i32 {
        self.scene
            .generated_chunk((x >> 4, z >> 4))
            .map_or(self.min_y(), |chunk| chunk.heightmaps.get(kind, (x & 15) as usize, (z & 15) as usize))
    }

    fn biome(&self, pos: BlockPos) -> BiomeId {
        self.noise_biome(pos.x >> 2, pos.y >> 2, pos.z >> 2)
    }

    fn noise_biome(&self, qx: i32, qy: i32, qz: i32) -> BiomeId {
        self.scene.noise_biome((qx, qy, qz)).unwrap_or(BiomeId(0))
    }

    fn raw_brightness(&mut self, pos: BlockPos, darkening: i32) -> i32 {
        (self.sky_brightness(pos) - darkening).max(self.block_brightness(pos))
    }

    fn sky_brightness(&mut self, pos: BlockPos) -> i32 {
        i32::from(self.light.get((pos.x, pos.y, pos.z)))
    }

    fn block_brightness(&mut self, pos: BlockPos) -> i32 {
        i32::from(self.light.get_block((pos.x, pos.y, pos.z)))
    }

    fn sky_darken(&self) -> i32 {
        self.sky_darken
    }

    fn min_y(&self) -> i32 {
        self.stream.states.vertical_range().start
    }

    fn random(&mut self) -> &mut dyn RandomSource {
        self.random
    }

    /// `Mth.createInsecureUUID`.
    fn next_uuid(&mut self) -> [i32; 4] {
        let most = (self.entity_random.next_i64() & -61441) | 16384;
        let least = (self.entity_random.next_i64() & 0x3FFF_FFFF_FFFF_FFFF) | i64::MIN;
        [(most >> 32) as i32, most as i32, (least >> 32) as i32, least as i32]
    }

    fn add_entity(&mut self, entity: Tag) {
        self.spawned.push(entity);
    }
}
