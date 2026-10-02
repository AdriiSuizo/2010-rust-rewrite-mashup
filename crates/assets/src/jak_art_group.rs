//! Jak's animations as the player's own art groups (`*.go`, the game's
//! linked object files) hold them: each animation's playback speed and
//! artist frame numbering, and the motion of its align joint and the
//! placing of its prejoint frame by frame, which the GLB export leaves out.
use std::collections::HashMap;

/// One animation's numbering and motion.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtAnim {
    pub speed: f32,
    pub artist_base: f32,
    pub artist_step: f32,
    /// The align joint at each frame: translation (units) then rotation as
    /// a quaternion (x, y, z, w), in the animation's own frame.
    pub align: Option<Vec<[f32; 7]>>,
    /// The prejoint at each frame, which every bone of the mesh hangs
    /// from, as a column-major matrix (units), when it is not where the
    /// root is.
    pub prejoint: Option<Vec<[f32; 16]>>,
}

/// Every one of Jak's animations in a linked art group, by name.
pub fn read(bytes: &[u8]) -> HashMap<String, ArtAnim> {
    let mut out = HashMap::new();
    let Some(header) = u32_at(bytes, 4)
        .map(|h| h as usize)
        .filter(|&h| h < bytes.len())
    else {
        return out;
    };
    let seg = &bytes[header..];
    let prefix = b"jakb-";
    let mut names = HashMap::new();
    let mut i = 4;
    while i + prefix.len() <= seg.len() {
        if &seg[i..i + prefix.len()] != prefix {
            i += 1;
            continue;
        }
        let end = seg[i..]
            .iter()
            .position(|&b| !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
            .map_or(seg.len(), |n| i + n);
        if seg.get(end) == Some(&0)
            && u32_at(seg, i - 4) == Some((end - i) as u32)
            && let Ok(name) = std::str::from_utf8(&seg[i..end])
        {
            names.insert((i - 4) as u32, name.to_owned());
        }
        i = end.max(i + 1);
    }
    for at in (4..seg.len().saturating_sub(4)).step_by(4) {
        let Some(name) = u32_at(seg, at).and_then(|w| names.get(&w)) else {
            continue;
        };
        let anim = at - 4;
        let (Some(speed), Some(artist_base), Some(artist_step)) = (
            f32_at(seg, anim + 44),
            f32_at(seg, anim + 48),
            f32_at(seg, anim + 52),
        ) else {
            continue;
        };
        if 0.0 < speed
            && speed <= 4.0
            && 0.0 < artist_step
            && artist_step <= 8.0
            && artist_base.abs() < 1000.0
        {
            let matrices = u32_at(seg, anim + 32).and_then(|frames| matrices(seg, frames as usize));
            let (align, prejoint) = matrices.map_or((None, None), |(a, p)| (Some(a), p));
            out.insert(
                name.clone(),
                ArtAnim {
                    speed,
                    artist_base,
                    artist_step,
                    align,
                    prejoint,
                },
            );
        }
    }
    out
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    u32_at(bytes, at).map(f32::from_bits)
}

/// The align joint at each frame, and the prejoint's matrices unless they
/// are all the identity, from the compressed animation: one fixed block,
/// then a block per frame, the whole optionally LZO compressed. The two
/// are full matrices, each either in the fixed block (it does not move) or
/// in each frame's block, align first, at the start of the 64-bit data.
#[allow(clippy::type_complexity)]
fn matrices(seg: &[u8], control: usize) -> Option<(Vec<[f32; 7]>, Option<Vec<[f32; 16]>>)> {
    let head = u32_at(seg, control)?;
    let frames = (head & 0xffff) as usize;
    let compressed = (head >> 16) & 1 != 0;
    let fixed_qwc = u32_at(seg, control + 4)? as usize;
    let frame_qwc = u32_at(seg, control + 8)? as usize;
    let fixed_at = u32_at(seg, control + 12)? as usize;
    if frames == 0 || frames > 4096 || fixed_qwc < 5 {
        return None;
    }
    let unpacked;
    let (fixed, frame_blocks): (&[u8], Vec<&[u8]>) = if compressed {
        let size = (fixed_qwc + frames * frame_qwc) * 16;
        let mut packed = lzo1x(seg.get(fixed_at..)?, size)?;
        // The packed stream may stop short of the last frame's padding.
        packed.resize(size, 0);
        unpacked = packed;
        let base = fixed_qwc * 16;
        (
            &unpacked[..base],
            (0..frames)
                .map(|i| &unpacked[base + i * frame_qwc * 16..base + (i + 1) * frame_qwc * 16])
                .collect(),
        )
    } else {
        let fixed = seg.get(fixed_at..fixed_at + fixed_qwc * 16)?;
        let mut blocks = Vec::with_capacity(frames);
        for i in 0..frames {
            let at = u32_at(seg, control + 16 + 4 * i)? as usize;
            blocks.push(seg.get(at..at + frame_qwc * 16)?);
        }
        (fixed, blocks)
    };
    let matrix_bits = u32_at(fixed, 60)?;
    let fixed_64 = u32_at(fixed, 64)? as usize;
    let read = |bytes: &[u8], at: usize| -> Option<[f32; 16]> {
        let m: Vec<f32> = (0..16)
            .map(|k| f32_at(bytes, at + 4 * k))
            .collect::<Option<_>>()?;
        m.try_into().ok()
    };
    let align_varies = matrix_bits & 1 != 0;
    let prejoint_varies = matrix_bits & 2 != 0;
    let fixed_prejoint = if prejoint_varies {
        None
    } else {
        let skip = if align_varies { 0 } else { 64 };
        Some(read(fixed, 80 + fixed_64 + skip)?)
    };
    let mut out = Vec::with_capacity(frames);
    let mut prejoints = Vec::with_capacity(frames);
    for block in frame_blocks {
        let frame_64 = 16 + u32_at(block, 0)? as usize;
        let m = if align_varies {
            read(block, frame_64)?
        } else {
            read(fixed, 80 + fixed_64)?
        };
        prejoints.push(match fixed_prejoint {
            Some(p) => p,
            None => read(block, frame_64 + if align_varies { 64 } else { 0 })?,
        });
        let col = |r: usize| {
            bevy::math::Vec3::new(m[4 * r], m[4 * r + 1], m[4 * r + 2])
                .normalize_or(bevy::math::Vec3::ZERO)
        };
        let rot = bevy::math::Mat3::from_cols(col(0), col(1), col(2));
        let q = bevy::math::Quat::from_mat3(&rot).normalize();
        out.push([m[12], m[13], m[14], q.x, q.y, q.z, q.w]);
    }
    let identity = bevy::math::Mat4::IDENTITY.to_cols_array();
    let moved = prejoints
        .iter()
        .any(|p| p.iter().zip(&identity).any(|(a, b)| (a - b).abs() > 1e-4));
    Some((out, moved.then_some(prejoints)))
}

/// LZO1X decompression, as the game unpacks animations it keeps packed.
fn lzo1x(src: &[u8], out_len: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(out_len);
    let mut ip = 0usize;
    let next = |ip: &mut usize| -> Option<usize> {
        let b = *src.get(*ip)?;
        *ip += 1;
        Some(usize::from(b))
    };
    let run = |ip: &mut usize, base: usize| -> Option<usize> {
        let mut n = 0;
        while *src.get(*ip)? == 0 {
            n += 255;
            *ip += 1;
        }
        Some(n + base + next(ip)?)
    };
    let mut t = next(&mut ip)?;
    let mut state;
    if t > 17 {
        t -= 17;
        out.extend_from_slice(src.get(ip..ip + t)?);
        ip += t;
        state = t.min(4);
        t = next(&mut ip)?;
    } else {
        state = 0;
    }
    while out.len() < out_len && ip <= src.len() {
        let (len, dist, tail);
        if t >= 64 {
            dist = ((t >> 2) & 7) + (next(&mut ip)? << 3) + 1;
            len = (t >> 5) + 1;
            tail = t & 3;
        } else if t >= 32 {
            let l = if t & 31 == 0 {
                run(&mut ip, 31)?
            } else {
                t & 31
            };
            len = l + 2;
            let d = next(&mut ip)? | (next(&mut ip)? << 8);
            dist = (d >> 2) + 1;
            tail = d & 3;
        } else if t >= 16 {
            let l = if t & 7 == 0 { run(&mut ip, 7)? } else { t & 7 };
            len = l + 2;
            let d = next(&mut ip)? | (next(&mut ip)? << 8);
            let far = ((t & 8) << 11) + (d >> 2);
            if far == 0 {
                break;
            }
            dist = far + 16384;
            tail = d & 3;
        } else if state == 0 {
            let l = if t == 0 { run(&mut ip, 15)? } else { t } + 3;
            out.extend_from_slice(src.get(ip..ip + l)?);
            ip += l;
            state = 4;
            t = next(&mut ip)?;
            continue;
        } else if state == 4 {
            dist = (t >> 2) + (next(&mut ip)? << 2) + 2049;
            len = 3;
            tail = t & 3;
        } else {
            dist = (t >> 2) + (next(&mut ip)? << 2) + 1;
            len = 2;
            tail = t & 3;
        }
        if dist > out.len() {
            return None;
        }
        for _ in 0..len {
            let b = out[out.len() - dist];
            out.push(b);
        }
        out.extend_from_slice(src.get(ip..ip + tail)?);
        ip += tail;
        state = tail;
        t = if ip < src.len() { next(&mut ip)? } else { 0 };
    }
    out.truncate(out_len);
    Some(out)
}
