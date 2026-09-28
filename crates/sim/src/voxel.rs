//! Collision against a Minecraft world. While a Minecraft map is loaded, world
//! traces against the map it stands in for test block collision shapes
//! instead of that map's brushes and meshes.
//!
//! Minecraft space is blocks with Y up. A map unit point `(X, Y, Z)` is block
//! point `origin + (X, Z, -Y) / BLOCK`: X stays east, Z up becomes Y up, and
//! the left-handed-looking swap of Y and Z is a proper rotation.
use std::collections::HashMap;
use std::sync::RwLock;

use crate::world::SimBrush;

/// Map units per block: the 70-unit soldier stands about two blocks tall,
/// and a block is below the 39-unit jump, so one block can be jumped onto.
pub const BLOCK: f32 = 36.0;
/// Pulled back from every hit, as IW4 traces keep off surfaces.
const SURFACE_CLIP_EPSILON: f32 = 0.125;
const SOLID: u32 = 1;
/// `SURF_TYPE` bits for a stone-like surface.
const STONE_SURFACE: u32 = 17 << 20;

/// One chunk column of shape ids, x fastest then z then y.
pub struct VoxelChunk {
    pub min_y: i32,
    pub height: i32,
    pub shapes: Vec<u16>,
}

/// The block world traces run against, and where it sits in map space.
#[derive(Default)]
pub struct VoxelWorld {
    brushes: usize,
    origin: [f64; 3],
    chunks: HashMap<(i32, i32), VoxelChunk>,
    /// Collision boxes of each shape id, in block space `[min, max]`; id 0 is
    /// empty.
    shapes: Vec<Vec<[f32; 6]>>,
}

static WORLD: RwLock<Option<VoxelWorld>> = RwLock::new(None);

/// What the authoritative game did to the block world this tick, for the
/// world's owner to apply.
#[derive(Clone, Copy, Debug)]
pub enum VoxelEvent {
    /// A bullet struck this block.
    Shot { block: [i32; 3] },
    /// An explosion went off here, in blocks.
    Explosion { center: [f64; 3] },
}

static EVENTS: std::sync::Mutex<Vec<VoxelEvent>> = std::sync::Mutex::new(Vec::new());

/// A bullet impact at map point `end` on a surface facing `normal`: the
/// block behind the surface.
pub fn push_shot(end: [f32; 3], normal: [f32; 3]) {
    let Ok(world) = WORLD.read() else {
        return;
    };
    let Some(world) = world.as_ref() else {
        return;
    };
    let p = to_block(world.origin, end);
    // Into the surface, past the trace's pull-back.
    let n = [f64::from(normal[0]), f64::from(normal[2]), -f64::from(normal[1])];
    let block = std::array::from_fn(|k| (p[k] - n[k] * 0.05).floor() as i32);
    if let Ok(mut events) = EVENTS.lock() {
        events.push(VoxelEvent::Shot { block });
    }
}

/// An explosion at map point `origin`.
pub fn push_explosion(origin: [f32; 3]) {
    let Ok(world) = WORLD.read() else {
        return;
    };
    let Some(world) = world.as_ref() else {
        return;
    };
    let center = to_block(world.origin, origin);
    if let Ok(mut events) = EVENTS.lock() {
        events.push(VoxelEvent::Explosion { center });
    }
}

pub fn take_events() -> Vec<VoxelEvent> {
    EVENTS.lock().map(|mut events| std::mem::take(&mut *events)).unwrap_or_default()
}

/// Sets one block's collision shape id, as `set_chunk` lays them out.
pub fn set_block_shape(x: i32, y: i32, z: i32, shape: u16) {
    if let Ok(mut world) = WORLD.write()
        && let Some(world) = world.as_mut()
        && let Some(chunk) = world.chunks.get_mut(&(x >> 4, z >> 4))
    {
        let ly = y - chunk.min_y;
        if ly >= 0 && ly < chunk.height {
            let index = ((ly * 16 + (z & 15)) * 16 + (x & 15)) as usize;
            if let Some(slot) = chunk.shapes.get_mut(index) {
                *slot = shape;
            }
        }
    }
}

/// Starts replacing world collision for the map whose brush table is
/// `brushes`, with the block world's `origin` block at map origin.
pub fn activate(brushes: &[SimBrush], origin: [f64; 3], shapes: Vec<Vec<[f32; 6]>>) {
    if let Ok(mut world) = WORLD.write() {
        *world = Some(VoxelWorld {
            brushes: brushes.as_ptr() as usize,
            origin,
            chunks: HashMap::new(),
            shapes,
        });
    }
}

pub fn deactivate() {
    if let Ok(mut world) = WORLD.write() {
        *world = None;
    }
    let _ = take_events();
}

/// Adds shape ids to the table and returns the first new id.
pub fn add_shapes(more: Vec<Vec<[f32; 6]>>) -> Option<u16> {
    let mut world = WORLD.write().ok()?;
    let world = world.as_mut()?;
    let first = world.shapes.len() as u16;
    world.shapes.extend(more);
    Some(first)
}

pub fn set_chunk(x: i32, z: i32, chunk: VoxelChunk) {
    if let Ok(mut world) = WORLD.write()
        && let Some(world) = world.as_mut()
    {
        world.chunks.insert((x, z), chunk);
    }
}

pub fn remove_chunk(x: i32, z: i32) {
    if let Ok(mut world) = WORLD.write()
        && let Some(world) = world.as_mut()
    {
        world.chunks.remove(&(x, z));
    }
}

/// Whether traces against `brushes` go to the block world.
pub(crate) fn active_for(brushes: &[SimBrush]) -> bool {
    WORLD
        .read()
        .ok()
        .is_some_and(|world| world.as_ref().is_some_and(|w| w.brushes == brushes.as_ptr() as usize))
}

/// Whether any block world is standing in for a map.
pub fn active() -> bool {
    WORLD.read().ok().is_some_and(|world| world.is_some())
}

/// Map point to block point.
pub fn to_block(origin: [f64; 3], p: [f32; 3]) -> [f64; 3] {
    let s = f64::from(BLOCK);
    [
        origin[0] + f64::from(p[0]) / s,
        origin[1] + f64::from(p[2]) / s,
        origin[2] - f64::from(p[1]) / s,
    ]
}

/// Block point to map point.
pub fn to_map(origin: [f64; 3], b: [f64; 3]) -> [f32; 3] {
    let s = f64::from(BLOCK);
    [
        ((b[0] - origin[0]) * s) as f32,
        (-(b[2] - origin[2]) * s) as f32,
        ((b[1] - origin[1]) * s) as f32,
    ]
}

impl VoxelWorld {
    fn shape_at(&self, x: i32, y: i32, z: i32) -> &[[f32; 6]] {
        let Some(chunk) = self.chunks.get(&(x >> 4, z >> 4)) else {
            return &[];
        };
        let ly = y - chunk.min_y;
        if ly < 0 || ly >= chunk.height {
            return &[];
        }
        let index = ((ly * 16 + (z & 15)) * 16 + (x & 15)) as usize;
        let id = chunk.shapes.get(index).copied().unwrap_or(0);
        self.shapes.get(usize::from(id)).map_or(&[], Vec::as_slice)
    }
}

struct Sweep {
    first: Option<(f64, [f64; 3])>,
    start_solid: bool,
    end_solid: bool,
}

impl VoxelWorld {
    /// A block-space box `[lo, hi]` about `a` moved to `b`: the first hit as
    /// a fraction of the move and its normal, or whether it starts inside.
    fn sweep(&self, a: [f64; 3], b: [f64; 3], lo: [f64; 3], hi: [f64; 3], test_start: bool) -> Sweep {
        const INSIDE: f64 = 1e-4;
        let delta: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
        let reach_lo: [i32; 3] = std::array::from_fn(|k| (a[k].min(b[k]) + lo[k] - 1.0).floor() as i32);
        let reach_hi: [i32; 3] = std::array::from_fn(|k| (a[k].max(b[k]) + hi[k] + 1.0).floor() as i32);
        let mut out = Sweep {
            first: None,
            start_solid: false,
            end_solid: false,
        };
        let mut best_t = f64::MAX;
        for y in reach_lo[1]..=reach_hi[1] {
            for z in reach_lo[2]..=reach_hi[2] {
                for x in reach_lo[0]..=reach_hi[0] {
                    for shape in self.shape_at(x, y, z) {
                        // The shape grown by the moving box.
                        let bmin = [
                            f64::from(x) + f64::from(shape[0]) - hi[0],
                            f64::from(y) + f64::from(shape[1]) - hi[1],
                            f64::from(z) + f64::from(shape[2]) - hi[2],
                        ];
                        let bmax = [
                            f64::from(x) + f64::from(shape[3]) - lo[0],
                            f64::from(y) + f64::from(shape[4]) - lo[1],
                            f64::from(z) + f64::from(shape[5]) - lo[2],
                        ];
                        let inside = |p: [f64; 3]| {
                            (0..3).all(|k| p[k] > bmin[k] + INSIDE && p[k] < bmax[k] - INSIDE)
                        };
                        if inside(a) {
                            if test_start {
                                out.start_solid = true;
                                out.end_solid |= inside(b);
                            }
                            continue;
                        }
                        let (mut t_in, mut t_out, mut axis) = (0.0f64, 1.0f64, None);
                        let mut hit = true;
                        for k in 0..3 {
                            if delta[k].abs() < 1e-12 {
                                if a[k] <= bmin[k] || a[k] >= bmax[k] {
                                    hit = false;
                                    break;
                                }
                                continue;
                            }
                            let (t0, t1) = ((bmin[k] - a[k]) / delta[k], (bmax[k] - a[k]) / delta[k]);
                            let (near, far) = (t0.min(t1), t0.max(t1));
                            if near > t_in {
                                t_in = near;
                                axis = Some(k);
                            }
                            t_out = t_out.min(far);
                            if t_in >= t_out {
                                hit = false;
                                break;
                            }
                        }
                        if hit
                            && let Some(k) = axis
                            && t_in < best_t
                        {
                            best_t = t_in;
                            let mut normal = [0.0; 3];
                            normal[k] = -delta[k].signum();
                            out.first = Some((t_in, normal));
                        }
                    }
                }
            }
        }
        out
    }
}

/// A box swept from `start` to `end` in map space against the block world.
pub(crate) fn trace(
    start: [f32; 3],
    end: [f32; 3],
    mins: [f32; 3],
    maxs: [f32; 3],
) -> trace_iw4::Trace {
    let open = trace_iw4::Trace {
        fraction: 1.0,
        endpos: end,
        ..trace_iw4::Trace::default()
    };
    let Ok(guard) = WORLD.read() else {
        return open;
    };
    let Some(world) = guard.as_ref() else {
        return open;
    };
    let s = f64::from(BLOCK);
    let a = to_block(world.origin, start);
    let b = to_block(world.origin, end);
    // The box in block space: map X, Z and -Y extents.
    let lo = [
        f64::from(mins[0]) / s,
        f64::from(mins[2]) / s,
        -f64::from(maxs[1]) / s,
    ];
    let hi = [
        f64::from(maxs[0]) / s,
        f64::from(maxs[2]) / s,
        -f64::from(mins[1]) / s,
    ];
    let delta = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    // Long traces (bullets) are tested in short segments, nearest first, so
    // the cells tested stay near the ray.
    let segments = (length / 4.0).ceil().max(1.0) as usize;
    let mut best_t = 1.0f64;
    let mut best_normal = [0.0f64; 3];
    let mut start_solid = false;
    let mut end_solid = false;
    for segment in 0..segments {
        let (t0, t1) = (
            segment as f64 / segments as f64,
            (segment + 1) as f64 / segments as f64,
        );
        let sa: [f64; 3] = std::array::from_fn(|k| a[k] + delta[k] * t0);
        let sb: [f64; 3] = std::array::from_fn(|k| a[k] + delta[k] * t1);
        let hit = world.sweep(sa, sb, lo, hi, segment == 0);
        if segment == 0 && hit.start_solid {
            start_solid = true;
            end_solid = segments == 1 && hit.end_solid;
            break;
        }
        if let Some((t, normal)) = hit.first {
            best_t = t0 + (t1 - t0) * t;
            best_normal = normal;
            break;
        }
    }
    if start_solid {
        return trace_iw4::Trace {
            fraction: 0.0,
            endpos: start,
            startsolid: 1,
            allsolid: u8::from(end_solid),
            contents: SOLID,
            surface_flags: STONE_SURFACE,
            ..trace_iw4::Trace::default()
        };
    }
    if best_t >= 1.0 {
        return open;
    }
    let pulled = if length > 0.0 {
        (best_t - f64::from(SURFACE_CLIP_EPSILON) / (length * s)).max(0.0)
    } else {
        0.0
    };
    let fraction = pulled as f32;
    let endpos = std::array::from_fn(|k| start[k] + (end[k] - start[k]) * fraction);
    let n = best_normal;
    trace_iw4::Trace {
        fraction,
        endpos,
        normal: [n[0] as f32, -(n[2] as f32), n[1] as f32],
        contents: SOLID,
        surface_flags: STONE_SURFACE,
        walkable: u8::from(n[1] > 0.7),
        ..trace_iw4::Trace::default()
    }
}
