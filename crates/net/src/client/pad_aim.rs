//! Aiming with a controller as MW2's console builds aim: turn rates that
//! blend from the hip to down the sight and scale with the view's zoom,
//! accelerating to full; and aim assist over visible enemies within the
//! weapon's own ranges: slowdown and lock-on in a box round the
//! crosshair, and auto aim onto a target when the sight comes up.
use super::input::ClientActionInput;

/// Turn rates at full deflection, degrees a second, from the hip and down
/// the sight.
const TURN_YAW: f32 = 260.0;
const TURN_PITCH: f32 = 90.0;
const TURN_YAW_ADS: f32 = 90.0;
const TURN_PITCH_ADS: f32 = 55.0;
/// How fast the turn rate grows towards the stick's, degrees a second
/// squared.
const TURN_ACCEL: f32 = 1200.0;

/// Slowdown: the turn scale over a target, from the hip and down the sight.
const SLOWDOWN_HIP: f32 = 0.4;
const SLOWDOWN_ADS: f32 = 0.5;
/// Regions round the crosshair, in the 640 by 480 virtual screen.
const SLOWDOWN_REGION: [f32; 2] = [90.0, 90.0];
const LOCKON_REGION: [f32; 2] = [90.0, 90.0];
const AUTOAIM_REGION: [f32; 2] = [160.0, 120.0];
/// Lock-on follows a target's motion this strongly, once a stick is past
/// this deflection.
const LOCKON_STRENGTH: f32 = 0.6;
const LOCKON_DEFLECTION: f32 = 0.05;
/// Auto aim closes on its target at this rate.
const AUTOAIM_LERP: f32 = 40.0;

/// Ranges for weapons whose data sets none.
const FALLBACK_RANGE: f32 = 1500.0;

/// Something aim assist can aim at: its key, its box in map space, the
/// point to aim at and its velocity.
#[derive(Clone, Copy, Debug)]
pub struct AimTarget {
    pub key: u64,
    pub mins: [f32; 3],
    pub maxs: [f32; 3],
    pub aim: [f32; 3],
    pub velocity: [f32; 3],
}

/// The view and weapon a frame's aim works from.
#[derive(Clone, Copy, Debug)]
pub struct AimView {
    pub eye: [f32; 3],
    /// Pitch and yaw, degrees.
    pub angles: [f32; 2],
    pub velocity: [f32; 3],
    /// 0 at the hip to 1 down the sight.
    pub ads_lerp: f32,
    /// The view's zoom against a 65 degree field of view.
    pub fov_scale: f32,
    pub ranges: weapon_iw4::AimAssistRanges,
    pub dt: f32,
}

/// A target projected onto the screen: its clip-space rectangle.
struct OnScreen {
    target: AimTarget,
    min: [f32; 2],
    max: [f32; 2],
    dist_sqr: f32,
    crosshair_sqr: f32,
}

fn axes(angles: [f32; 2]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let (sp, cp) = angles[0].to_radians().sin_cos();
    let (sy, cy) = angles[1].to_radians().sin_cos();
    let forward = [cp * cy, cp * sy, -sp];
    let right = [sy, -cy, 0.0];
    let up = [sp * cy, sp * sy, cp];
    (forward, right, up)
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn angles_to(eye: [f32; 3], point: [f32; 3]) -> [f32; 2] {
    let d = [point[0] - eye[0], point[1] - eye[1], point[2] - eye[2]];
    let flat = (d[0] * d[0] + d[1] * d[1]).sqrt();
    [-d[2].atan2(flat).to_degrees(), d[1].atan2(d[0]).to_degrees()]
}

fn angle_delta(to: f32, from: f32) -> f32 {
    (to - from + 540.0).rem_euclid(360.0) - 180.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The targets on screen, nearest the crosshair first.
fn project(view: &AimView, targets: &[AimTarget]) -> Vec<OnScreen> {
    let (forward, right, up) = axes(view.angles);
    let tan_x = (32.5f32.to_radians().tan() * view.fov_scale).max(1e-3);
    let tan_y = tan_x * 9.0 / 16.0;
    let mut out: Vec<OnScreen> = targets
        .iter()
        .filter_map(|target| {
            let mut min = [f32::MAX; 2];
            let mut max = [f32::MIN; 2];
            for corner in 0..8 {
                let p = [
                    if corner & 1 == 0 { target.mins[0] } else { target.maxs[0] },
                    if corner & 2 == 0 { target.mins[1] } else { target.maxs[1] },
                    if corner & 4 == 0 { target.mins[2] } else { target.maxs[2] },
                ];
                let d = [p[0] - view.eye[0], p[1] - view.eye[1], p[2] - view.eye[2]];
                let depth = dot(d, forward);
                if depth <= 0.0 {
                    continue;
                }
                let x = dot(d, right) / (depth * tan_x);
                let y = -dot(d, up) / (depth * tan_y);
                min = [min[0].min(x), min[1].min(y)];
                max = [max[0].max(x), max[1].max(y)];
            }
            if max[0] <= min[0] || max[1] <= min[1] || min[0] > 1.0 || min[1] > 1.0 || max[0] < -1.0 || max[1] < -1.0 {
                return None;
            }
            let min = [min[0].clamp(-1.0, 1.0), min[1].clamp(-1.0, 1.0)];
            let max = [max[0].clamp(-1.0, 1.0), max[1].clamp(-1.0, 1.0)];
            let centre = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
            let d = [target.aim[0] - view.eye[0], target.aim[1] - view.eye[1], target.aim[2] - view.eye[2]];
            Some(OnScreen {
                target: *target,
                min,
                max,
                dist_sqr: dot(d, d),
                crosshair_sqr: centre[0] * centre[0] + centre[1] * centre[1],
            })
        })
        .collect();
    out.sort_by(|a, b| a.crosshair_sqr.total_cmp(&b.crosshair_sqr));
    out
}

/// The nearest target to the crosshair within `range` whose rectangle
/// meets the region, `region` virtual pixels round the crosshair.
fn best<'a>(screen: &'a [OnScreen], range: f32, region: [f32; 2], scale: f32) -> Option<&'a OnScreen> {
    let half = [region[0] / 640.0 * scale, region[1] / 480.0 * scale];
    screen.iter().find(|s| {
        s.dist_sqr <= range * range && s.min[0] <= half[0] && s.max[0] >= -half[0] && s.min[1] <= half[1] && s.max[1] >= -half[1]
    })
}

/// This frame's controller turn, pitch and yaw in degrees, into
/// `input.pad_look_delta`.
pub fn pad_look_frame(input: &mut ClientActionInput, view: &AimView, targets: &[AimTarget], ads: bool) {
    let dt = view.dt;
    let ads_lerp = view.ads_lerp.clamp(0.0, 1.0);
    let sensitivity = input.pad_sensitivity * lerp(1.0, input.pad_ads_sensitivity, ads_lerp);
    let zoom = view.fov_scale.clamp(0.02, 2.0);
    let max_yaw = lerp(TURN_YAW, TURN_YAW_ADS, ads_lerp) * zoom * sensitivity;
    let max_pitch = lerp(TURN_PITCH, TURN_PITCH_ADS, ads_lerp) * zoom * sensitivity;
    let wanted = [-input.pad_look[1] * max_pitch, -input.pad_look[0] * max_yaw];

    // The turn rate grows towards the stick's and drops at once.
    for axis in 0..2 {
        let rate = &mut input.pad_turn_rate[axis];
        let goal = wanted[axis];
        if goal.abs() > rate.abs() && goal.signum() == rate.signum() || *rate == 0.0 && goal != 0.0 {
            let step = TURN_ACCEL * zoom * dt;
            *rate = if goal > *rate { (*rate + step).min(goal) } else { (*rate - step).max(goal) };
        } else {
            *rate = goal;
        }
    }
    let mut delta = [input.pad_turn_rate[0] * dt, input.pad_turn_rate[1] * dt];

    let mode = input.pad_aim_assist;
    if mode == 0 {
        input.pad_lockon = None;
        input.pad_autoaim = None;
        input.pad_was_ads = ads;
        input.pad_look_delta = delta;
        return;
    }
    let screen = project(view, targets);
    let range = |hip: f32, ads: f32| {
        let hip = if hip > 0.0 { hip } else { FALLBACK_RANGE };
        let ads = if ads > 0.0 { ads } else { hip };
        lerp(hip, ads, ads_lerp)
    };
    let assist_range = range(view.ranges.hip, view.ranges.ads);

    // Slowdown over a target.
    if best(&screen, assist_range, SLOWDOWN_REGION, 1.0).is_some() {
        let slow = lerp(SLOWDOWN_HIP, SLOWDOWN_ADS, ads_lerp);
        delta[0] *= slow;
        delta[1] *= slow;
    }

    // Lock-on: while a stick is moving, the view follows the target's
    // motion against the player's.
    let kept = input
        .pad_lockon
        .and_then(|key| screen.iter().find(|s| s.target.key == key))
        .filter(|s| best(std::slice::from_ref(*s), assist_range, LOCKON_REGION, 1.0).is_some());
    let lock = kept.or_else(|| best(&screen, assist_range, LOCKON_REGION, 1.0));
    input.pad_lockon = lock.map(|s| s.target.key);
    if let Some(lock) = lock
        && input.pad_deflection > LOCKON_DEFLECTION
    {
        let t = lock.target;
        let moved = [
            t.aim[0] + (t.velocity[0] - view.velocity[0]) * dt,
            t.aim[1] + (t.velocity[1] - view.velocity[1]) * dt,
            t.aim[2] + (t.velocity[2] - view.velocity[2]) * dt,
        ];
        let now = angles_to(view.eye, t.aim);
        let next = angles_to(view.eye, moved);
        delta[0] += LOCKON_STRENGTH * angle_delta(next[0], now[0]);
        delta[1] += LOCKON_STRENGTH * angle_delta(next[1], now[1]);
    }

    // Auto aim: raising the sight picks the target nearest the crosshair
    // within the weapon's auto aim range, and the view closes on it.
    if mode == 2 && ads && !input.pad_was_ads {
        let auto_range = if view.ranges.auto_aim > 0.0 { view.ranges.auto_aim } else { FALLBACK_RANGE };
        input.pad_autoaim =
            best(&screen, auto_range, AUTOAIM_REGION, (1.0 / zoom).max(1.0)).map(|s| s.target.key);
    }
    if !ads {
        input.pad_autoaim = None;
    }
    if let Some(key) = input.pad_autoaim {
        match screen.iter().find(|s| s.target.key == key) {
            Some(s) => {
                let goal = angles_to(view.eye, s.target.aim);
                let step = (AUTOAIM_LERP * dt).min(1.0);
                let pitch = angle_delta(goal[0], view.angles[0]);
                let yaw = angle_delta(goal[1], view.angles[1]);
                delta[0] += pitch * step;
                delta[1] += yaw * step;
                if pitch.abs() < 0.25 && yaw.abs() < 0.25 {
                    input.pad_autoaim = None;
                }
            }
            None => input.pad_autoaim = None,
        }
    }
    input.pad_was_ads = ads;
    input.pad_look_delta = delta;
}
