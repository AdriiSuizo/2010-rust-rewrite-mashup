//! Jak Mode against Jak 3's own numbers: each expectation is a tuning value
//! or the closed form of a formula from the game's code, written out here
//! rather than read back from `jak_mode`, so a transcription slip fails.
use jak_mode::glam::Vec3;
use jak_mode::*;

const M: f32 = 4096.0;
const TICKS_PER_FRAME: i64 = 5;

fn quad(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> [Tri; 2] {
    [
        Tri {
            v: [a, b, c],
            pat: Pat::default(),
        },
        Tri {
            v: [a, c, d],
            pat: Pat::default(),
        },
    ]
}

/// A floor at y = 0, and a wall facing -z at `wall` meters when given.
fn ground(wall: Option<f32>) -> TriangleGrid {
    let s = 2000.0 * M;
    let mut tris = quad(
        Vec3::new(-s, 0.0, -s),
        Vec3::new(-s, 0.0, s),
        Vec3::new(s, 0.0, s),
        Vec3::new(s, 0.0, -s),
    )
    .to_vec();
    if let Some(z) = wall {
        let (z, h) = (z * M, 20.0 * M);
        tris.extend(quad(
            Vec3::new(-s, 0.0, z),
            Vec3::new(-s, h, z),
            Vec3::new(s, h, z),
            Vec3::new(s, 0.0, z),
        ));
    }
    TriangleGrid::new(tris, 4.0 * M)
}

fn idle() -> PadInput {
    PadInput::default()
}

fn press(b: u32) -> PadInput {
    PadInput {
        held: b,
        pressed: b,
        ..Default::default()
    }
}

fn hold(b: u32) -> PadInput {
    PadInput {
        held: b,
        ..Default::default()
    }
}

fn forward() -> PadInput {
    PadInput {
        left: [0.0, 1.0],
        ..Default::default()
    }
}

fn run(jak: &mut Jak, world: &mut TriangleGrid, input: PadInput, frames: usize) {
    for _ in 0..frames {
        jak.step(&input, world, None);
    }
}

fn standing(world: &mut TriangleGrid) -> Jak {
    let mut jak = Jak::new(Vec3::ZERO, 0.0);
    run(&mut jak, world, idle(), 60);
    assert_eq!(jak.state, State::Stance);
    jak
}

fn boarding(world: &mut TriangleGrid) -> Jak {
    let mut jak = standing(world);
    jak.request_board();
    run(&mut jak, world, idle(), 60);
    assert_eq!(jak.state, State::BoardStance);
    jak
}

/// Highest the root rises over where it took off, in meters, for a jump
/// whose button stays down for `held` frames after the press.
fn jump_apex(jak: &mut Jak, world: &mut TriangleGrid, held: usize) -> f32 {
    let base = jak.trans().y;
    jak.step(&press(button::X), world, None);
    let mut top = jak.trans().y;
    for frame in 0..240 {
        let input = if frame < held {
            hold(button::X)
        } else {
            idle()
        };
        jak.step(&input, world, None);
        top = top.max(jak.trans().y);
    }
    (top - base) / M
}

fn xz_speed(v: Vec3) -> f32 {
    (v.x * v.x + v.z * v.z).sqrt()
}

#[test]
fn units() {
    assert_eq!(math::meters(1.0), 4096.0);
    assert_eq!(math::seconds(1.0), 300);
    assert_eq!(math::degrees(360.0), 65536.0);
    assert_eq!(math::FRAME_TICKS, TICKS_PER_FRAME);
}

#[test]
fn board_surfaces_multiply_into_stone() {
    let walk = surface::Surface::mult(&surface::board::WALK, &surface::STONE);
    assert_eq!(walk.seek0, 0.5 * 153600.0);
    assert_eq!(walk.seek90, 0.5 * 153600.0);
    assert_eq!(walk.seek180, 0.5 * 256000.0);
    assert_eq!(walk.target_speed, 102400.0);
    assert_eq!(walk.transv_max, 143360.0);
}

/// Full forward reads 127/128 off the pad. Each grounded frame the board
/// adds `seek * lerp(0.4, 1, stick, 0.3, 1)` along its heading, then drags
/// by `1 - seek / target` of a frame, with seek 76800 and target 102400.
#[test]
fn board_thrust_follows_its_seek_curve() {
    let mut world = ground(None);
    let mut jak = boarding(&mut world);
    let seek_step = 0.5 * 153600.0 / 60.0;
    let drag = 1.0 - seek_step / 102400.0;
    let push = 0.4 + 0.6 * ((127.0 / 128.0 - 0.3) / 0.7);
    let mut speed = xz_speed(jak.velocity());
    run(&mut jak, &mut world, forward(), 1);
    speed = speed.max(xz_speed(jak.velocity()));
    for _ in 0..600 {
        jak.step(&forward(), &mut world, None);
        let expected = (speed + seek_step * push) * drag;
        speed = xz_speed(jak.velocity());
        assert!((speed - expected).abs() < 0.5, "{speed} vs {expected}");
    }
    let terminal = push * seek_step * drag / (1.0 - drag);
    assert!(
        (speed - terminal).abs() < 0.01 * terminal,
        "{speed} vs {terminal}"
    );
}

/// Left alone the board still pushes at 0.4 of its seek: it cruises.
#[test]
fn board_cruises_with_the_stick_released() {
    let mut world = ground(None);
    let mut jak = boarding(&mut world);
    run(&mut jak, &mut world, idle(), 900);
    let cruise = 0.4 * 102400.0 * (1.0 - 1280.0 / 102400.0);
    let speed = xz_speed(jak.velocity());
    assert!(
        (speed - cruise).abs() < 0.005 * cruise,
        "{speed} vs {cruise}"
    );
}

#[test]
fn board_jump_reaches_its_heights() {
    let mut world = ground(None);
    let mut jak = boarding(&mut world);
    let held = jump_apex(&mut jak, &mut world, 60);
    assert!((held - 3.5).abs() < 0.01, "{held}");
    run(&mut jak, &mut world, idle(), 60);
    let tapped = jump_apex(&mut jak, &mut world, 0);
    assert!((tapped - 1.01).abs() < 0.01, "{tapped}");
}

/// On foot the same heights, less the 0.7 m the jump raises the collision
/// by (and 0.1 m back at the low end).
#[test]
fn foot_jump_reaches_its_heights() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    let held = jump_apex(&mut jak, &mut world, 60);
    assert!((held - (3.5 - 0.7)).abs() < 0.01, "{held}");
    run(&mut jak, &mut world, idle(), 60);
    let tapped = jump_apex(&mut jak, &mut world, 0);
    assert!((tapped - (1.01 - 0.6)).abs() < 0.01, "{tapped}");
}

/// The hop onto the board is timed to land 0.66 s after it starts.
#[test]
fn board_get_on_lands_after_its_hop() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    jak.request_board();
    jak.step(&idle(), &mut world, None);
    assert_eq!(jak.state, State::BoardGetOn);
    let start = jak.time;
    while jak.state == State::BoardGetOn && jak.time - start < 600 {
        jak.step(&idle(), &mut world, None);
    }
    let took = jak.time - start;
    assert!(
        (took - math::seconds(0.66)).abs() <= 2 * TICKS_PER_FRAME,
        "{took}"
    );
}

fn fire_times(jak: &mut Jak, world: &mut TriangleGrid, frames: usize) -> Vec<i64> {
    let mut times = Vec::new();
    for _ in 0..frames {
        jak.step(&press(button::R1), world, None);
        for event in jak.take_events() {
            if let Event::Fire { .. } = event {
                times.push(jak.time);
            }
        }
    }
    times
}

/// The Blaster is up 0.1 s after the first press and fires then; held
/// presses fire again every 96 ticks, on the first frame past them.
#[test]
fn blaster_fires_on_its_delay() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    let pressed = jak.time + TICKS_PER_FRAME;
    let times = fire_times(&mut jak, &mut world, 120);
    assert_eq!(times.first().copied(), Some(pressed + math::seconds(0.1)));
    let frame_after_delay = (96 + TICKS_PER_FRAME - 1) / TICKS_PER_FRAME * TICKS_PER_FRAME;
    assert!(times.len() >= 5);
    for pair in times.windows(2) {
        assert_eq!(pair[1] - pair[0], frame_after_delay);
    }
}

/// 200 m/s, gone after 3 s with nothing in the way.
#[test]
fn yellow_shot_flies_and_times_out() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    jak.step(&press(button::R1), &mut world, None);
    while jak.projectiles.is_empty() {
        jak.step(&idle(), &mut world, None);
    }
    let spawned = jak.projectiles[0].spawn_time;
    let mut last = jak.projectiles[0].trans;
    let step = 819200.0 / 60.0;
    while !jak.projectiles.is_empty() {
        jak.step(&idle(), &mut world, None);
        if let Some(shot) = jak.projectiles.first() {
            assert!((shot.trans.distance(last) - step).abs() < 1.0);
            assert!(shot.tail.distance(shot.trans) <= 16.0 * M + 1.0);
            last = shot.trans;
        }
    }
    assert_eq!(jak.time - spawned, math::seconds(3.0));
}

/// The shot stops on the wall it meets and settles half a meter back along
/// its tail, its 0.2 m sphere touching the wall.
#[test]
fn yellow_shot_stops_on_a_wall() {
    let wall = 30.0;
    let mut world = ground(Some(wall));
    let mut jak = standing(&mut world);
    let mut hit = None;
    for frame in 0..120 {
        let input = if frame == 0 {
            press(button::R1)
        } else {
            idle()
        };
        jak.step(&input, &mut world, None);
        for event in jak.take_events() {
            if let Event::Impact(h) = event {
                hit = Some(h);
            }
        }
        if hit.is_some() {
            break;
        }
    }
    let hit = hit.expect("the shot reaches the wall");
    assert!((hit.surface.z - wall * M).abs() < 1.0);
    assert!(hit.normal.distance(Vec3::NEG_Z) < 1e-4);
    assert!(
        (hit.pos.z - (wall * M - 819.2 - 2048.0)).abs() < 1.0,
        "{}",
        hit.pos.z
    );
    assert_eq!(hit.damage, 2.0);
    assert_eq!(hit.actor, None);
}

/// A wall inside the muzzle's reach takes the shot on the frame it fires.
#[test]
fn yellow_shot_point_blank() {
    let mut world = ground(Some(2.0));
    let mut jak = standing(&mut world);
    let mut fired = None;
    let mut struck = None;
    for _ in 0..60 {
        jak.step(&press(button::R1), &mut world, None);
        for event in jak.take_events() {
            match event {
                Event::Fire { .. } => fired = fired.or(Some(jak.time)),
                Event::Impact(_) => struck = struck.or(Some(jak.time)),
                _ => {}
            }
        }
        if struck.is_some() {
            break;
        }
    }
    assert!(fired.is_some());
    assert_eq!(fired, struck);
    assert!(jak.projectiles.is_empty());
}

/// A sphere swept into a face meets it one radius out; past an edge it meets
/// the edge; a solid sweep passes through a face from behind.
#[test]
fn sphere_sweeps_meet_faces_edges_and_skip_backs() {
    let floor = quad(
        Vec3::new(-M, 0.0, -M),
        Vec3::new(-M, 0.0, M),
        Vec3::new(M, 0.0, M),
        Vec3::new(M, 0.0, -M),
    );
    let mut world = TriangleGrid::new(floor.to_vec(), 4.0 * M);
    let mut cache = CollideCache::default();
    let r = 0.7 * M;
    let start = Vec3::new(0.0, 2.0 * M, 0.0);
    let down = Vec3::new(0.0, -3.0 * M, 0.0);
    cache.fill_line_sphere(&mut world, start, down, r);
    let face = cache.probe_line_sphere(start, down, r).expect("face");
    assert!((face.u - (2.0 * M - r) / (3.0 * M)).abs() < 1e-5);
    assert!(face.normal.distance(Vec3::Y) < 1e-6);
    assert!(face.intersect.y.abs() < 1e-3);

    let beside = Vec3::new(M + 0.5 * r, 2.0 * M, 0.0);
    cache.fill_line_sphere(&mut world, beside, down, r);
    let edge = cache.probe_line_sphere(beside, down, r).expect("edge");
    let drop = 2.0 * M - (r * r - 0.25 * r * r).sqrt();
    assert!((edge.u - drop / (3.0 * M)).abs() < 1e-4, "{}", edge.u);
    assert!((edge.intersect.x - M).abs() < 1e-2);

    let below = Vec3::new(0.0, -2.0 * M, 0.0);
    let up = Vec3::new(0.0, 3.0 * M, 0.0);
    cache.fill_line_sphere(&mut world, below, up, r);
    assert!(cache.probe_line_sphere(below, up, r).is_none());
}

/// A turn is limited to the rate's share of a frame, or 5/frames of what is
/// left when that is less.
#[test]
fn smooth_rotation_limits_the_turn() {
    let from = Vec3::Z;
    let to = Vec3::X;
    let rate = math::degrees(180.0);
    let q = math::smooth_rotation(from, to, rate, 30, Vec3::Y);
    let turned = math::to_radians(rate / 60.0);
    assert!(((q * from).angle_between(from) - turned).abs() < 1e-4);
    let q = math::smooth_rotation(from, to, math::degrees(3600.0), 30, Vec3::Y);
    let share = std::f32::consts::FRAC_PI_2 * 5.0 / 30.0;
    assert!(((q * from).angle_between(from) - share).abs() < 1e-4);
}

/// Riding along a wall at a shallow angle glances off it: the board turns
/// to run along the wall and never passes it.
#[test]
fn board_glances_off_a_wall() {
    let wall = 30.0;
    let mut world = ground(Some(wall));
    let mut jak = boarding(&mut world);
    let slant = PadInput {
        left: [-0.3, 0.95],
        ..Default::default()
    };
    let mut glanced = false;
    for _ in 0..600 {
        jak.step(&slant, &mut world, None);
        glanced |= jak.take_events().contains(&Event::BoardGlance);
        assert!(jak.trans().z < wall * M);
    }
    assert!(glanced);
}

fn stick(held: u32, pressed: u32, x: f32, z: f32) -> PadInput {
    PadInput {
        held: held | pressed,
        pressed,
        left: [x, z],
        ..Default::default()
    }
}

/// A held board jump, until Jak rises past 1.2 m: high enough for a trick.
fn rising(jak: &mut Jak, world: &mut TriangleGrid) {
    jak.step(&press(button::X), world, None);
    while jak.trans().y < 1.2 * M {
        jak.step(&hold(button::X), world, None);
        assert!(jak.velocity().y > 0.0);
    }
}

/// L1's press is only noted after the frame's checks, so a grab out of a
/// plain jump starts the frame after it, if L1 is still down. Then it plays
/// its whole animation: letting go of L1 does not cut the nose flip short. It ends when the flip
/// reaches its artist frame 20, and only then.
#[test]
fn board_trick_lasts_its_animation() {
    let mut world = ground(None);
    let mut jak = boarding(&mut world);
    rising(&mut jak, &mut world);
    jak.step(&stick(0, button::L1, 0.0, 1.0), &mut world, None);
    assert!(matches!(jak.state, State::BoardJump { .. }));
    jak.step(&stick(button::L1, 0, 0.0, 1.0), &mut world, None);
    assert_eq!(jak.state, State::BoardTricky);
    assert!(jak.chan.is(anim::BOARD_NOSEFLIP));
    let a = &jak.anims;
    let target = a.aframe(anim::BOARD_NOSEFLIP, 20.0);
    let speed = a.info(anim::BOARD_NOSEFLIP).speed;
    let frames = (target / speed).ceil() as usize;
    for frame in 1..frames {
        jak.step(&stick(0, 0, 0.0, 1.0), &mut world, None);
        assert_eq!(jak.state, State::BoardTricky, "frame {frame}");
    }
    jak.step(&stick(0, 0, 0.0, 1.0), &mut world, None);
    assert_eq!(jak.state, State::BoardFalling);
    assert_eq!(jak.trace.why, "code");
}

/// Moving the stick from across to forward while L1's press still counts
/// cancels the kick trick into the grab trick, through the same per-frame
/// check that starts tricks out of a jump.
#[test]
fn board_trick_cancels_into_another() {
    let mut world = ground(None);
    let mut jak = boarding(&mut world);
    rising(&mut jak, &mut world);
    jak.step(&stick(0, button::L1, 0.6, 0.0), &mut world, None);
    assert_eq!(jak.state, State::BoardTrickx);
    jak.step(&stick(button::L1, 0, 0.0, 0.8), &mut world, None);
    assert_eq!(jak.state, State::BoardTricky);
    assert_eq!(jak.trace.prev, "board-trickx");
    assert_eq!(jak.trace.why, "trans");
}

/// Chained tricks climb. A trick's jump adds what is left of the rise, so
/// each new one started while still rising launches faster than the last.
/// Once the kick trick's exit has cleared the grab's latch, every fresh L1
/// press with the stick part way forward restarts the grab, never letting
/// one finish and stop the rise.
fn chained_climb(side: f32) -> (f32, State) {
    let mut world = ground(None);
    let mut jak = boarding(&mut world);
    rising(&mut jak, &mut world);
    jak.step(&stick(0, button::L1, side, 0.0), &mut world, None);
    jak.step(&stick(button::L1, 0, 0.0, 0.8), &mut world, None);
    for frame in 0..360 {
        let input = match frame % 4 {
            0 => stick(0, button::L1, 0.0, 0.8),
            3 => stick(0, 0, 0.0, 0.8),
            _ => stick(button::L1, 0, 0.0, 0.8),
        };
        jak.step(&input, &mut world, None);
    }
    (jak.trace.peak / M, jak.state)
}

#[test]
fn board_tricks_chained_while_rising_keep_climbing() {
    let (peak, state) = chained_climb(0.6);
    assert!(peak > 50.0, "{peak}");
    assert_eq!(state, State::BoardTricky);
}

/// With the stick fully across, the kick trick latches a full sideways
/// push, and the grab can never start after it: no climb.
#[test]
fn board_tricks_with_full_tilt_do_not_climb() {
    let (peak, _) = chained_climb(1.0);
    assert!(peak < 6.0, "{peak}");
}

/// Animations two frames long, so a trick plays out while Jak still rises.
fn quick(names: &[&str]) -> std::sync::Arc<Anims> {
    let info = anim::AnimInfo {
        frames: 3,
        speed: 1.0,
        artist_base: 0.0,
        artist_step: 10.0,
    };
    std::sync::Arc::new(Anims::from_rows(names.iter().map(|n| (*n, info))))
}

/// A grab that plays out cuts the rise; a kick trick that plays out keeps
/// it.
#[test]
fn board_grab_played_out_stops_the_rise() {
    let mut world = ground(None);
    let mut jak = boarding(&mut world);
    jak.anims = quick(&["jakb-board-noseflip"]);
    rising(&mut jak, &mut world);
    jak.step(&stick(0, button::L1, 0.0, 1.0), &mut world, None);
    jak.step(&stick(button::L1, 0, 0.0, 1.0), &mut world, None);
    assert_eq!(jak.state, State::BoardTricky);
    let mut up = 0.0;
    while jak.state == State::BoardTricky {
        up = jak.velocity().y;
        jak.step(&idle(), &mut world, None);
    }
    assert!(up > 0.0, "{up}");
    assert_eq!(jak.state, State::BoardFalling);
    let fall = jak.velocity().y;
    assert!(fall <= 0.0 && fall > -245760.0 / 60.0 - 1.0, "{fall}");
}

#[test]
fn board_kick_trick_played_out_keeps_the_rise() {
    let mut world = ground(None);
    let mut jak = boarding(&mut world);
    jak.anims = quick(&[
        "jakb-board-kickflip-a",
        "jakb-board-kickflip-b",
        "jakb-board-kickflip-c",
    ]);
    rising(&mut jak, &mut world);
    jak.step(&stick(0, button::L1, -1.0, 0.0), &mut world, None);
    assert_eq!(jak.state, State::BoardTrickx);
    while jak.state == State::BoardTrickx {
        jak.step(&idle(), &mut world, None);
    }
    assert_eq!(jak.state, State::BoardFalling);
    assert!(jak.velocity().y > 0.0, "{}", jak.velocity().y);
}

/// The moves that travel with their animation, given a plain motion: the
/// punch and the roll straight ahead, the uppercut straight up, the roll's
/// flip in an arc. With the player's files the real motion replaces it.
fn moving_anims() -> std::sync::Arc<Anims> {
    use jak_mode::glam::Quat;
    let track = |f: &dyn Fn(f32) -> Vec3| AlignTrack {
        trans: (0..21).map(|i| f(i as f32 / 20.0)).collect(),
        quat: vec![Quat::IDENTITY; 21],
    };
    let anims = Anims::nominal()
        .with_align(
            "jakb-attack-punch",
            track(&|t| Vec3::new(0.0, 0.0, 9.0 * M * t)),
        )
        .with_align(
            "jakb-duck-roll",
            track(&|t| Vec3::new(0.0, 0.0, 6.0 * M * t)),
        )
        .with_align(
            "jakb-attack-uppercut",
            track(&|t| Vec3::new(0.0, 5.0 * M * t, 0.0)),
        )
        .with_align(
            "jakb-roll-flip",
            track(&|t| {
                Vec3::new(
                    0.0,
                    3.2969 * M * (std::f32::consts::PI * t).sin(),
                    12.5 * M * t,
                )
            }),
        )
        .with_align(
            "jakb-turn-around",
            AlignTrack {
                trans: vec![Vec3::ZERO; 21],
                quat: (0..21)
                    .map(|i| Quat::from_rotation_y(std::f32::consts::PI * i as f32 / 20.0))
                    .collect(),
            },
        );
    std::sync::Arc::new(anims)
}

fn mover(world: &mut TriangleGrid) -> Jak {
    let mut jak = standing(world);
    jak.anims = moving_anims();
    jak
}

/// Frames until `state` matches, up to `limit`, holding `input`.
fn until(
    jak: &mut Jak,
    world: &mut TriangleGrid,
    input: PadInput,
    limit: usize,
    f: impl Fn(&State) -> bool,
) -> usize {
    for frame in 0..limit {
        if f(&jak.state) {
            return frame;
        }
        jak.step(&input, world, None);
    }
    assert!(f(&jak.state), "still {:?}", jak.state);
    limit
}

/// Square punches; X while the punch still carries Jak forward turns it
/// into the uppercut, which then rises on its own animation.
#[test]
fn punch_then_x_is_uppercut() {
    let mut world = ground(None);
    let mut jak = mover(&mut world);
    jak.step(&press(button::SQUARE), &mut world, None);
    assert_eq!(jak.state, State::RunningAttack);
    assert_eq!(jak.attack.danger, Some(Danger::Punch));
    run(&mut jak, &mut world, hold(button::SQUARE), 8);
    assert!(
        jak.control.ctrl_xz_vel > 4096.0,
        "{}",
        jak.control.ctrl_xz_vel
    );
    jak.step(&press(button::X), &mut world, None);
    assert!(
        matches!(jak.state, State::AttackUppercut { .. }),
        "{:?}",
        jak.state
    );
    assert_eq!(jak.trace.prev, "running-attack");
    until(&mut jak, &mut world, idle(), 120, |s| {
        matches!(s, State::AttackUppercutJump { .. })
    });
    let base = jak.trans().y;
    let mut top = base;
    for _ in 0..90 {
        jak.step(&idle(), &mut world, None);
        top = top.max(jak.trans().y);
    }
    assert!(top - base > 2.0 * M, "{}", (top - base) / M);
}

/// The uppercut's animation past artist frame 12, circle spins in the air.
#[test]
fn uppercut_then_circle_is_air_spin() {
    let mut world = ground(None);
    let mut jak = mover(&mut world);
    jak.step(&press(button::SQUARE), &mut world, None);
    run(&mut jak, &mut world, hold(button::SQUARE), 8);
    jak.step(&press(button::X), &mut world, None);
    until(&mut jak, &mut world, idle(), 120, |s| {
        matches!(s, State::AttackUppercutJump { .. })
    });
    while jak.chan.aframe_num(&jak.anims) < 12.0 {
        jak.step(&idle(), &mut world, None);
        assert!(matches!(jak.state, State::AttackUppercutJump { .. }));
    }
    jak.step(&press(button::CIRCLE), &mut world, None);
    assert_eq!(
        jak.state,
        State::AttackAir {
            from: AirFrom::Uppercut
        }
    );
    assert_eq!(jak.attack.danger, Some(Danger::SpinAir));
}

/// Circle in the air spins; landing ends it.
#[test]
fn jump_then_circle_is_air_spin() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    jak.step(&press(button::X), &mut world, None);
    run(&mut jak, &mut world, hold(button::X), 10);
    jak.step(&press(button::CIRCLE), &mut world, None);
    assert_eq!(
        jak.state,
        State::AttackAir {
            from: AirFrom::Jump
        }
    );
    until(&mut jak, &mut world, idle(), 240, |s| {
        matches!(s, State::HitGround { .. } | State::Stance)
    });
}

/// Square in the air, near the top of the jump, dives: down at the dive's
/// speed, dangerous on the way, and a landing of its own.
#[test]
fn jump_then_square_is_dive() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    jak.step(&press(button::X), &mut world, None);
    while jak.velocity().y >= 26624.0 {
        jak.step(&hold(button::X), &mut world, None);
    }
    jak.step(&press(button::SQUARE), &mut world, None);
    assert!(matches!(jak.state, State::Flop { .. }), "{:?}", jak.state);
    until(&mut jak, &mut world, idle(), 240, |s| {
        matches!(s, State::FlopHitGround { .. })
    });
}

/// X while still rising slowly in a jump jumps again.
#[test]
fn jump_then_x_is_double_jump() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    jak.step(&press(button::X), &mut world, None);
    while jak.velocity().y >= 12288.0 {
        jak.step(&idle(), &mut world, None);
    }
    jak.step(&press(button::X), &mut world, None);
    assert!(
        matches!(jak.state, State::DoubleJump { .. }),
        "{:?}",
        jak.state
    );
}

/// Ducking with the stick let go, X crouches and leaps 7 m.
#[test]
fn duck_then_x_is_high_jump() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    jak.step(&hold(button::L1), &mut world, None);
    assert!(
        matches!(jak.state, State::DuckStance { .. }),
        "{:?}",
        jak.state
    );
    run(&mut jak, &mut world, hold(button::L1), 5);
    let base = jak.trans().y;
    jak.step(&press(button::X | button::L1), &mut world, None);
    assert!(
        matches!(
            jak.state,
            State::DuckHighJump {
                kind: HighJump::Duck,
                ..
            }
        ),
        "{:?}",
        jak.state
    );
    until(&mut jak, &mut world, idle(), 120, |s| {
        matches!(s, State::DuckHighJumpJump { .. })
    });
    let mut top = base;
    for _ in 0..240 {
        jak.step(&idle(), &mut world, None);
        top = top.max(jak.trans().y);
    }
    assert!(
        ((top - base) / M - 7.0).abs() < 0.05,
        "{}",
        (top - base) / M
    );
}

fn running(jak: &mut Jak, world: &mut TriangleGrid) {
    run(jak, world, forward(), 30);
    assert_eq!(jak.state, State::Walk);
}

/// L1 while running rolls; an X pressed during the roll is kept and turns
/// its end into the flip.
#[test]
fn roll_then_x_is_roll_flip() {
    let mut world = ground(None);
    let mut jak = mover(&mut world);
    running(&mut jak, &mut world);
    jak.step(
        &PadInput {
            pressed: button::L1,
            held: button::L1,
            ..forward()
        },
        &mut world,
        None,
    );
    assert_eq!(jak.state, State::Roll);
    run(&mut jak, &mut world, forward(), 3);
    jak.step(
        &PadInput {
            pressed: button::X,
            held: button::X,
            ..forward()
        },
        &mut world,
        None,
    );
    assert_eq!(jak.state, State::Roll);
    until(&mut jak, &mut world, forward(), 120, |s| {
        matches!(s, State::RollFlip { .. })
    });
    assert_eq!(jak.trace.why, "code");
    let start = jak.trans();
    until(&mut jak, &mut world, forward(), 240, |s| {
        !matches!(s, State::RollFlip { .. })
    });
    assert!((jak.trans() - start).length() > 4.0 * M);
}

/// Without X the roll ends ducking; X in the tenth of a second after still
/// flips.
#[test]
fn x_just_after_a_roll_still_flips() {
    let mut world = ground(None);
    let mut jak = mover(&mut world);
    running(&mut jak, &mut world);
    jak.step(
        &PadInput {
            pressed: button::L1,
            held: button::L1,
            ..forward()
        },
        &mut world,
        None,
    );
    until(&mut jak, &mut world, forward(), 120, |s| *s != State::Roll);
    assert!(
        matches!(
            jak.state,
            State::DuckStance { .. } | State::DuckWalk { .. } | State::Walk
        ),
        "{:?}",
        jak.state
    );
    jak.step(
        &PadInput {
            pressed: button::X,
            held: button::X,
            ..forward()
        },
        &mut world,
        None,
    );
    assert!(
        matches!(jak.state, State::RollFlip { .. }),
        "{:?}",
        jak.state
    );
}

/// A press a few frames before it can act still acts: X pressed in the
/// double jump, which takes no third jump, jumps again on landing.
#[test]
fn a_press_just_before_landing_is_kept() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    jak.step(&press(button::X), &mut world, None);
    while jak.velocity().y >= 12288.0 {
        jak.step(&idle(), &mut world, None);
    }
    jak.step(&press(button::X), &mut world, None);
    assert!(matches!(jak.state, State::DoubleJump { .. }));
    while jak.trans().y > 0.4 * M || jak.velocity().y > 0.0 {
        jak.step(&idle(), &mut world, None);
    }
    jak.step(&press(button::X), &mut world, None);
    assert!(matches!(jak.state, State::DoubleJump { .. }));
    until(&mut jak, &mut world, idle(), 14, |s| {
        matches!(s, State::Jump { .. })
    });
    assert_eq!(jak.trace.prev, "hit-ground");
}

/// Mid-spin, X jumps out of it.
#[test]
fn spin_then_x_jumps() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    jak.step(&press(button::CIRCLE), &mut world, None);
    assert_eq!(jak.state, State::Attack);
    run(&mut jak, &mut world, idle(), 5);
    jak.step(&press(button::X), &mut world, None);
    assert!(matches!(jak.state, State::Jump { .. }), "{:?}", jak.state);
}

/// An actor in reach of the spin: struck once for the spin's 3 hit points,
/// however many frames the spin touches it.
struct Dummy {
    at: Vec3,
}

impl ActorWorld for Dummy {
    fn actor_hit(&mut self, _: Vec3, _: Vec3, _: f32) -> Option<(u64, f32, Vec3)> {
        None
    }

    fn actors_touching(&mut self, center: Vec3, radius: f32) -> Vec<u64> {
        if (center - self.at).length() < radius + 0.5 * M {
            vec![7]
        } else {
            Vec::new()
        }
    }
}

#[test]
fn spin_strikes_each_target_once() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    let mut dummy = Dummy {
        at: jak.trans() + Vec3::new(0.0, 1.5 * M, 2.0 * M),
    };
    jak.take_events();
    jak.step(&press(button::CIRCLE), &mut world, Some(&mut dummy));
    for _ in 0..60 {
        jak.step(&idle(), &mut world, Some(&mut dummy));
    }
    let strikes: Vec<_> = jak
        .take_events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Strike {
                actor: Some(a),
                damage,
                ..
            } => Some((a, damage)),
            _ => None,
        })
        .collect();
    assert_eq!(strikes, vec![(7, 3.0)]);
}

fn back() -> PadInput {
    PadInput {
        left: [0.0, -1.0],
        ..Default::default()
    }
}

/// Pulling the stick back at a full run skids Jak round: the skid turns
/// him on its animation and he sets off the other way at a walk's speed.
#[test]
fn reversing_at_a_run_skids_round() {
    let mut world = ground(None);
    let mut jak = mover(&mut world);
    run(&mut jak, &mut world, forward(), 60);
    assert_eq!(jak.state, State::Walk);
    assert!(jak.velocity().z > 9.0 * M, "{}", jak.velocity().z / M);
    until(&mut jak, &mut world, back(), 5, |s| *s == State::TurnAround);
    until(&mut jak, &mut world, back(), 60, |s| *s == State::Walk);
    let v = jak.velocity();
    assert!(v.z < -9.0 * M, "{}", v.z / M);
}

/// Walking is not running: a slow walk never skids.
#[test]
fn reversing_at_a_walk_does_not_skid() {
    let mut world = ground(None);
    let mut jak = mover(&mut world);
    let slow = PadInput {
        left: [0.0, 0.5],
        ..Default::default()
    };
    run(&mut jak, &mut world, slow, 60);
    for _ in 0..30 {
        jak.step(&back(), &mut world, None);
        assert_ne!(jak.state, State::TurnAround);
    }
}

/// A floor at y = 0 and a ledge `height` meters up that ends at z = 2 m.
fn ledge(height: f32) -> TriangleGrid {
    let s = 2000.0 * M;
    let h = height * M;
    let mut tris = quad(
        Vec3::new(-s, 0.0, -s),
        Vec3::new(-s, 0.0, s),
        Vec3::new(s, 0.0, s),
        Vec3::new(s, 0.0, -s),
    )
    .to_vec();
    tris.extend(quad(
        Vec3::new(-s, h, -s),
        Vec3::new(-s, h, 2.0 * M),
        Vec3::new(s, h, 2.0 * M),
        Vec3::new(s, h, -s),
    ));
    TriangleGrid::new(tris, 4.0 * M)
}

fn run_off(height: f32) -> (Jak, Vec<Event>) {
    let mut world = ledge(height);
    let mut jak = Jak::new(Vec3::new(0.0, height * M, 0.0), 0.0);
    run(&mut jak, &mut world, idle(), 60);
    assert_eq!(jak.state, State::Stance);
    jak.take_events();
    let mut events = Vec::new();
    for _ in 0..600 {
        jak.step(&forward(), &mut world, None);
        events.extend(jak.take_events());
        if jak.control.on_surface() && jak.trans().y < 1.0 * M {
            jak.step(&forward(), &mut world, None);
            events.extend(jak.take_events());
            break;
        }
    }
    (jak, events)
}

/// Over thirty meters a landing is hard: it costs a unit of health for the
/// first thirty and one more for every twenty after, and Jak is down until
/// the painful landing has played.
#[test]
fn a_fall_over_thirty_meters_lands_hard() {
    let (mut jak, events) = run_off(40.0);
    assert!(
        matches!(jak.state, State::HitGroundHard { .. }),
        "{:?}",
        jak.state
    );
    assert!(
        events.contains(&Event::HardLanding { health: 1.0 }),
        "{events:?}"
    );
    let mut world = ledge(40.0);
    for _ in 0..10 {
        jak.step(&press(button::X), &mut world, None);
        assert!(matches!(jak.state, State::HitGroundHard { .. }));
    }
    until(&mut jak, &mut world, idle(), 200, |s| *s == State::Stance);

    let (_, events) = run_off(55.0);
    assert!(
        events.contains(&Event::HardLanding { health: 2.0 }),
        "{events:?}"
    );
}

#[test]
fn a_fall_under_thirty_meters_lands_unhurt() {
    let (jak, events) = run_off(20.0);
    assert!(!matches!(jak.state, State::HitGroundHard { .. }));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::HardLanding { .. }))
    );
}

/// At a run the walk cycle mixes all the way to the run and advances by
/// the ground covered over the run cycle's six and a quarter meters.
#[test]
fn running_mixes_the_run_cycle() {
    let mut world = ground(None);
    let mut jak = mover(&mut world);
    run(&mut jak, &mut world, forward(), 120);
    let mix = jak.chan.mix.expect("the walk cycle mixes");
    assert_eq!(mix.run, 1.0);
    let speed = jak.control.ctrl_xz_vel;
    let before = jak.chan.frame;
    jak.step(&forward(), &mut world, None);
    let max = jak.anims.max(jak.chan.anim);
    let step = (jak.chan.frame - before).rem_euclid(max);
    let want = speed / (6.25 * M) * jak.anims.info(jak.chan.anim).speed;
    assert!((step - want).abs() < 0.02, "{step} {want}");
}

/// Made-up timing for the flaps, numbered so the frames the swoop's
/// windows name fall inside them.
fn light_anims() -> std::sync::Arc<Anims> {
    let info = |base: f32| jak_mode::anim::AnimInfo {
        frames: 25,
        speed: 0.5,
        artist_base: base,
        artist_step: 1.5,
    };
    std::sync::Arc::new(Anims::from_rows([
        ("jakb-lightjak-swoop1", info(10.0)),
        ("jakb-lightjak-swoop2", info(40.0)),
        ("jakb-lightjak-swoop-fall", info(76.0)),
    ]))
}

/// A jump, then X again once the double jump takes it.
fn double_jump_press(jak: &mut Jak, world: &mut TriangleGrid) {
    jak.step(&press(button::X), world, None);
    while jak.velocity().y >= 12288.0 {
        jak.step(&idle(), world, None);
    }
    jak.step(&press(button::X), world, None);
}

/// Steps until a flap lifts Jak, and the climb it left him with.
fn until_flap(jak: &mut Jak, world: &mut TriangleGrid) -> f32 {
    for _ in 0..120 {
        let before = jak.velocity().y;
        jak.step(&idle(), world, None);
        if jak.velocity().y > before + 8192.0 {
            return jak.velocity().y;
        }
    }
    panic!("no flap: {:?}", jak.state);
}

fn l2_and(b: u32, pressed: u32) -> PadInput {
    PadInput {
        held: button::L2 | b,
        pressed,
        ..Default::default()
    }
}

/// Holding the power button and pressing X changes Jak into Light Jak
/// with the flight, wings and all.
fn light_jak(world: &mut TriangleGrid) -> Jak {
    let mut jak = standing(world);
    jak.anims = light_anims();
    jak.lightjak.eco = jak_mode::lightjak::ECO_MAX;
    run(&mut jak, world, l2_and(0, button::L2), 1);
    assert_eq!(jak.state, State::PowerJakGetOn);
    run(&mut jak, world, l2_and(0, 0), 5);
    jak.step(&l2_and(button::X, button::X), world, None);
    until(&mut jak, world, idle(), 60, |s| {
        matches!(s, State::LightJakGetOn { swoop: true })
    });
    until(&mut jak, world, idle(), 600, |s| *s == State::Stance);
    assert!(jak.light());
    assert!(jak.lightjak.swoop);
    assert!(jak.lightjak.wings.is_some());
    jak
}

#[test]
fn power_button_and_x_change_into_light_jak() {
    let mut world = ground(None);
    let jak = light_jak(&mut world);
    assert_eq!(jak.lightjak.eco, jak_mode::lightjak::ECO_MAX);
}

/// As Light Jak the double jump's press spreads the wings: the first flap
/// costs a unit of light eco and sets the climb to 1.4 of the flap's
/// 77824 units a second under a lighter pull; X again after half a second
/// flaps again for free, as hard as the time since allows; between flaps
/// he falls under the usual pull, X flaps once more, and the landing is
/// the swoop's own.
#[test]
fn light_jak_flies_on_the_double_jump() {
    let mut world = ground(None);
    let mut jak = light_jak(&mut world);
    let eco = jak.lightjak.eco;
    double_jump_press(&mut jak, &mut world);
    assert!(
        matches!(jak.state, State::LightJakSwoop { first: true, .. }),
        "{:?}",
        jak.state
    );
    assert_eq!(jak.lightjak.eco, eco - 1.0);
    let flap = 285.0 * 163840.0 / 600.0;
    let up = jak.velocity().y;
    assert!(
        up <= 1.4 * flap && up > 1.4 * flap - 163840.0 / 30.0,
        "{up}"
    );
    assert_eq!(jak.control.gravity_length, 163840.0);
    assert!(
        jak.lightjak
            .wings
            .is_some_and(|w| w.mode == jak_mode::lightjak::WingsMode::Use)
    );

    until(&mut jak, &mut world, idle(), 200, |s| {
        *s == State::LightJakSwoopFalling
    });
    assert_eq!(jak.control.gravity_length, 245760.0);
    assert!(
        jak.trans().y > 2.0 * M,
        "too low to flap: {}",
        jak.trans().y / M
    );
    jak.step(&press(button::X), &mut world, None);
    assert_eq!(
        jak.state,
        State::LightJakSwoop {
            first: false,
            held: 1.0
        }
    );
    let up = until_flap(&mut jak, &mut world);
    assert!(up <= flap && up > flap - 163840.0 / 30.0, "{up}");
    assert_eq!(jak.lightjak.eco, eco - 1.0);

    let held = 40;
    run(&mut jak, &mut world, idle(), held);
    jak.step(&press(button::X), &mut world, None);
    let State::LightJakSwoop {
        first: false,
        held: seconds,
    } = jak.state
    else {
        panic!("{:?}", jak.state);
    };
    let lift = ((seconds * seconds - 0.25) / 0.7).clamp(0.0, 1.0);
    let up = until_flap(&mut jak, &mut world);
    assert!(
        up <= lift * flap && up > lift * flap - 163840.0 / 30.0,
        "{up} {seconds}"
    );
    assert_eq!(jak.lightjak.eco, eco - 1.0);

    until(&mut jak, &mut world, idle(), 600, |s| {
        matches!(s, State::HitGround { .. })
    });
    assert!(jak.chan.is(jak_mode::anim::LIGHTJAK_SWOOP_LAND));
    assert!(jak.light());
}

/// Light Jak neither ducks nor rolls.
#[test]
fn light_jak_does_not_duck_or_roll() {
    let mut world = ground(None);
    let mut jak = light_jak(&mut world);
    run(&mut jak, &mut world, hold(button::L1), 20);
    assert_eq!(jak.state, State::Stance);
    run(&mut jak, &mut world, forward(), 30);
    jak.step(
        &PadInput {
            held: button::L1,
            pressed: button::L1,
            left: [0.0, 1.0],
            ..Default::default()
        },
        &mut world,
        None,
    );
    assert_ne!(jak.state, State::Roll);
}

/// A tap of the power button as Light Jak changes him back.
#[test]
fn power_button_tap_changes_back() {
    let mut world = ground(None);
    let mut jak = light_jak(&mut world);
    run(&mut jak, &mut world, l2_and(0, button::L2), 1);
    assert_eq!(jak.state, State::PowerJakGetOn);
    jak.step(&idle(), &mut world, None);
    assert_eq!(jak.state, State::LightJakGetOff);
    until(&mut jak, &mut world, idle(), 600, |s| *s == State::Stance);
    assert!(!jak.light());
    assert!(jak.lightjak.wings.is_none());
}

/// The flight's last eco: he keeps flying, and changes back on landing.
#[test]
fn light_jak_ends_when_the_eco_runs_out() {
    let mut world = ground(None);
    let mut jak = light_jak(&mut world);
    jak.lightjak.eco = 1.0;
    double_jump_press(&mut jak, &mut world);
    assert!(matches!(
        jak.state,
        State::LightJakSwoop { first: true, .. }
    ));
    assert_eq!(jak.lightjak.eco, 0.0);
    assert!(jak.light());
    until(&mut jak, &mut world, idle(), 600, |s| {
        *s == State::LightJakGetOff
    });
    until(&mut jak, &mut world, idle(), 600, |s| *s == State::Stance);
    assert!(!jak.light());
}

/// Without Light Jak the double jump is a double jump.
#[test]
fn no_flight_without_light_jak() {
    let mut world = ground(None);
    let mut jak = standing(&mut world);
    jak.lightjak.eco = jak_mode::lightjak::ECO_MAX;
    double_jump_press(&mut jak, &mut world);
    assert!(matches!(jak.state, State::DoubleJump { .. }));
}
