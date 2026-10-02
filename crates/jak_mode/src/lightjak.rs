//! Light Jak as far as his flight goes: changing into him and back on the
//! power button, the light eco both run on, and the swoop: wings spread on
//! a double jump's press, flapped again in the air, Jak falling between
//! flaps, the wings' own animation following his.
use crate::anim::{self, Anim, Anims, Channel, NumFunc};
use crate::control::{STANDARD_GRAVITY, STANDARD_GRAVITY_MAX, status};
use crate::math::*;
use crate::pad::button;
use crate::surface::{self, flag};
use crate::{Event, Jak, Push, State};

/// The light eco a flight's first flap costs, and a shot fired as Light
/// Jak.
pub const SWOOP_INC: f32 = 1.0;
/// A full light eco meter.
pub const ECO_MAX: f32 = 100.0;
const SWOOP_GRAVITY: f32 = 163840.0;
const SWOOP_GRAVITY_MAX: f32 = 245760.0;
/// Asking for the flight while changing.
const LATCH_SWOOP: u32 = 16;
const LONG_AGO: i64 = i64::MIN / 4;

/// Jak's animations the wings follow frame for frame; any other leaves
/// them folded.
const FOLLOWED: [Anim; 7] = [
    anim::LIGHTJAK_SWOOP1,
    anim::LIGHTJAK_SWOOP2,
    anim::LIGHTJAK_SWOOP_FALL,
    anim::LIGHTJAK_SWOOP_FALL_LOOP,
    anim::LIGHTJAK_GET_OFF,
    anim::LIGHTJAK_GET_ON_OUT,
    anim::LIGHTJAK_GET_ON_LAND,
];

const SWOOP_ANIMS: [Anim; 4] = [
    anim::LIGHTJAK_SWOOP1,
    anim::LIGHTJAK_SWOOP2,
    anim::LIGHTJAK_SWOOP_FALL,
    anim::LIGHTJAK_SWOOP_FALL_LOOP,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WingsMode {
    /// Folded on his back in their own stance.
    Idle,
    /// Following Jak's animation.
    Use,
    /// Folding away as Light Jak ends.
    Close,
}

/// How folded wings start: unfolding, already folded, or neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WingsStart {
    Open,
    Force,
    Plain,
}

/// The wings on Light Jak's back, with their own animation channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wings {
    pub mode: WingsMode,
    pub chan: Channel,
    pub push: Push,
    /// Drawn: not once folded away, nor while Jak plays an animation they
    /// have none for.
    pub shown: bool,
    start: WingsStart,
    lock: bool,
    pc: u8,
    arrived: bool,
}

impl Wings {
    fn new(start: WingsStart) -> Self {
        let mut wings = Self {
            mode: WingsMode::Idle,
            chan: Channel::default(),
            push: Push::default(),
            shown: true,
            start,
            lock: false,
            pc: 0,
            arrived: true,
        };
        wings.idle(start);
        wings
    }

    fn idle(&mut self, start: WingsStart) {
        self.mode = WingsMode::Idle;
        self.start = start;
        self.lock = start != WingsStart::Plain;
        self.pc = 0;
        self.arrived = true;
    }

    fn go(&mut self, mode: WingsMode) {
        self.mode = mode;
        self.pc = 0;
        self.arrived = true;
    }

    /// Asked to spread: from folded, they follow Jak.
    fn open(&mut self) {
        if self.mode == WingsMode::Idle {
            self.go(WingsMode::Use);
        }
    }

    fn end_mode(&mut self) {
        if self.mode != WingsMode::Close {
            self.go(WingsMode::Close);
        }
    }

    fn push(&mut self, now: i64, ticks: i64) {
        self.push = Push { tick: now, ticks };
    }

    /// A do-while wait on the wings' channel.
    fn wait(&mut self, anims: &Anims) -> bool {
        if self.arrived {
            return false;
        }
        self.chan.eval(anims);
        self.arrived = true;
        self.chan.done(anims)
    }

    /// One frame: the checks, the sequence, then copying Jak's pose while
    /// following him.
    fn step(&mut self, anims: &Anims, jak: &Channel, jak_push: Push, now: i64) {
        self.arrived = false;
        let followed = FOLLOWED.contains(&jak.anim);
        match self.mode {
            WingsMode::Idle if !self.lock && followed => self.go(WingsMode::Use),
            WingsMode::Use if !followed => self.idle(WingsStart::Plain),
            _ => {}
        }
        match self.mode {
            WingsMode::Idle => self.idle_code(anims, now),
            WingsMode::Use => {
                if self.pc == 0 {
                    self.push(now, seconds(0.1));
                    self.pc = 1;
                }
                match wing_anim(jak.anim) {
                    Some(anim) => {
                        self.chan = Channel { anim, ..*jak };
                        self.push = jak_push;
                        self.shown = true;
                    }
                    None => self.shown = false,
                }
            }
            WingsMode::Close => {
                if self.pc == 0 {
                    self.push(now, seconds(0.1));
                    self.chan
                        .set(anim::WINGS_LIGHTJAK_GET_OFF, NumFunc::seek(1.0), 0.0);
                    self.pc = 1;
                } else if self.pc == 1 && self.wait(anims) {
                    self.shown = false;
                    self.pc = 2;
                }
            }
        }
    }

    fn idle_code(&mut self, anims: &Anims, now: i64) {
        loop {
            match self.pc {
                0 => match self.start {
                    WingsStart::Open => {
                        let to = anims.aframe(anim::WINGS_LIGHTJAK_GET_ON_LAND, 45.0);
                        self.chan.set(
                            anim::WINGS_LIGHTJAK_GET_ON_LAND,
                            NumFunc::seek_to(to, 1.0),
                            0.0,
                        );
                        self.pc = 1;
                    }
                    WingsStart::Force => {
                        let max = anims.max(anim::WINGS_LIGHTJAK_STANCE);
                        self.chan
                            .set(anim::WINGS_LIGHTJAK_STANCE, NumFunc::Identity, max);
                        self.pc = 3;
                        return;
                    }
                    WingsStart::Plain => self.pc = 4,
                },
                1 => {
                    if !self.wait(anims) {
                        return;
                    }
                    self.chan.func = NumFunc::seek(1.0);
                    self.pc = 2;
                }
                2 => {
                    if self.chan.done(anims) {
                        self.pc = 4;
                        continue;
                    }
                    if !self.arrived {
                        self.chan.eval(anims);
                        self.arrived = true;
                        continue;
                    }
                    return;
                }
                3 => self.pc = 4,
                4 => {
                    self.lock = false;
                    self.push(now, seconds(0.1));
                    self.pc = 5;
                }
                5 => {
                    self.chan
                        .set(anim::WINGS_LIGHTJAK_STANCE, NumFunc::seek(1.0), 0.0);
                    self.pc = 6;
                }
                _ => {
                    if !self.wait(anims) {
                        return;
                    }
                    self.pc = 5;
                }
            }
        }
    }
}

/// The wings' animation to an animation of Jak's: its name with `wings-`
/// after Jak's prefix.
fn wing_anim(jak: Anim) -> Option<Anim> {
    let rest = jak.name().strip_prefix("jakb-")?;
    Anim::by_name(&format!("jakb-wings-{rest}"))
}

/// Light Jak's part of Jak.
#[derive(Clone, Debug, PartialEq)]
pub struct LightJak {
    /// Changed into Light Jak.
    pub on: bool,
    /// The flight is his: the wings come with the change.
    pub swoop: bool,
    /// Light eco, out of [`ECO_MAX`].
    pub eco: f32,
    /// When he last changed into Light Jak.
    pub start_time: i64,
    /// When the change back began, out of the moves that cannot stop for
    /// it; 0 when it has not.
    pub latch_out_time: i64,
    /// What the power button asked for while it was held.
    pub get_on_latch: u32,
    /// Letting go of the power button does not change him back.
    pub get_off_lock: bool,
    pub swoop_count: u32,
    /// How far the change has gone, 0 to 1.
    pub interp: f32,
    /// The upward speed a flap sets.
    pub swoop_impulse: f32,
    pub wings: Option<Wings>,
}

impl Default for LightJak {
    fn default() -> Self {
        Self {
            on: false,
            swoop: false,
            eco: 0.0,
            start_time: LONG_AGO,
            latch_out_time: 0,
            get_on_latch: 0,
            get_off_lock: false,
            swoop_count: 0,
            interp: 0.0,
            swoop_impulse: 0.0,
            wings: None,
        }
    }
}

impl LightJak {
    /// What the debug readout calls his state.
    pub fn describe(&self) -> String {
        let wings = self.wings.map_or("none".to_owned(), |w| {
            format!(
                "{:?} {}{}",
                w.mode,
                w.chan.anim.name().trim_start_matches("jakb-wings-"),
                if w.shown { "" } else { " (hidden)" }
            )
        });
        format!(
            "{}  eco {:.0}/{:.0}  flight {}  wings {}  swoops {}{}",
            if self.on { "on" } else { "off" },
            self.eco,
            ECO_MAX,
            if self.swoop { "yes" } else { "no" },
            wings,
            self.swoop_count,
            if self.latch_out_time != 0 {
                "  ending"
            } else {
                ""
            }
        )
    }
}

impl Jak {
    /// Light Jak, ready to act.
    pub fn light(&self) -> bool {
        self.lightjak.on
    }

    /// Holding the power button where Jak may change.
    pub(crate) fn want_to_powerjak(&self) -> bool {
        (self.pad.hold(button::L2) || self.lightjak.get_on_latch != 0)
            && !self.state.is_board()
            && self.time_elapsed(self.lightjak.start_time, seconds(0.05))
    }

    /// Able to change into Light Jak now.
    fn want_to_lightjak(&self) -> bool {
        !self.state.is_board()
            && self.time_elapsed(self.lightjak.start_time, seconds(0.05))
            && 0.0 < self.lightjak.eco
            && self.lightjak.latch_out_time == 0
    }

    fn spawn_wings(&mut self, start: WingsStart) {
        if self.lightjak.wings.is_none() {
            self.lightjak.wings = Some(Wings::new(start));
        }
    }

    /// Light Jak ends: at once, or by starting the change back the moves
    /// in progress finish first.
    pub(crate) fn lightjak_end_mode(&mut self, now: bool) {
        if !self.lightjak.on {
            return;
        }
        let l = &mut self.lightjak;
        if now {
            l.get_off_lock = false;
            l.on = false;
            if l.eco < 1.0 {
                l.eco = (l.eco - 1.0).max(0.0);
            }
            if !self
                .chan
                .is_any(&[anim::LIGHTJAK_GET_OFF, anim::LIGHTJAK_GET_ON])
                && self.chan.anim.name().starts_with("jakb-lightjak-")
            {
                self.chan.set(anim::STANCE_LOOP, NumFunc::Identity, 0.0);
            }
            let l = &mut self.lightjak;
            l.wings = None;
            l.latch_out_time = 0;
            self.events.push(Event::LightJak { on: false });
        } else if l.latch_out_time == 0 {
            l.latch_out_time = self.time;
            if let Some(w) = &mut l.wings {
                w.end_mode();
            }
        }
    }

    /// Every frame on foot: the power button's lock, the change back once
    /// the eco runs out, and the wings.
    pub(crate) fn lightjak_process(&mut self) {
        if !self.pad.hold(button::L2) && self.state != State::PowerJakGetOn {
            self.lightjak.get_off_lock = false;
        }
        if self.lightjak.latch_out_time != 0 {
            if self.time_elapsed(self.lightjak.latch_out_time, seconds(0.4)) {
                self.lightjak_end_mode(true);
            }
        } else if self.lightjak.on {
            let changing = matches!(
                self.state,
                State::LightJakGetOn { .. }
                    | State::LightJakGetOff
                    | State::LightJakSwoop { .. }
                    | State::LightJakSwoopFalling
            );
            if self.lightjak.eco < 1.0 && !changing && self.attack.danger.is_none() {
                if matches!(self.state, State::Stance | State::HitGround { .. }) {
                    self.pending = Some(State::LightJakGetOff);
                } else {
                    self.lightjak_end_mode(false);
                }
            }
        } else {
            self.lightjak.interp = 0.0;
        }
        let (chan, push, now) = (self.chan, self.push, self.time);
        if let Some(w) = &mut self.lightjak.wings {
            w.step(&self.anims, &chan, push, now);
        }
    }

    /// A double jump's press as Light Jak with the flight: a swoop instead.
    pub(crate) fn swoop_instead(&self) -> bool {
        let l = &self.lightjak;
        self.pad.pressed(button::X)
            && l.on
            && l.swoop
            && SWOOP_INC <= l.eco
            && self.time_elapsed(
                self.control.last_time_of_stuck,
                crate::target::STUCK_TIMEOUT,
            )
            && self.control.current.flags & (flag::NO_ATTACK | flag::NO_FEET) == 0
            && l.latch_out_time == 0
    }

    pub(crate) fn lightjak_enter(&mut self, state: &State) {
        match *state {
            State::LightJakGetOff => {
                self.control.mod_surface = surface::foot::LIGHTJAK_TRANS;
            }
            State::LightJakSwoop { first, held } => {
                let l = &mut self.lightjak;
                l.get_off_lock = true;
                self.spawn_wings(WingsStart::Open);
                let l = &mut self.lightjak;
                if first {
                    l.eco = (l.eco - SWOOP_INC).max(0.0);
                    if let Some(w) = &mut l.wings {
                        w.open();
                    }
                }
                l.swoop_count += 1;
                let lift = if first {
                    1.4
                } else {
                    lerp_scale(0.0, 1.0, held * held, 0.25, 0.95)
                };
                l.swoop_impulse = (1.0 / 600.0) * (285.0 * lift) * SWOOP_GRAVITY;
                let c = &mut self.control;
                c.gravity_max = SWOOP_GRAVITY_MAX;
                c.gravity_length = SWOOP_GRAVITY;
                c.status &= !(status::ON_SURFACE | status::ON_GROUND | status::TOUCH_SURFACE);
                c.mod_surface = surface::foot::LIGHTJAK_SWOOP;
                c.last_trans_any_surf = c.trans;
                self.foot.sliding_start_time = self.time;
                self.events.push(Event::Flap);
            }
            State::LightJakSwoopFalling => self.control.mod_surface = surface::JUMP,
            _ => {}
        }
    }

    pub(crate) fn lightjak_exit(&mut self, state: &State, next: &State) {
        match *state {
            State::PowerJakGetOn => {
                self.lightjak.get_on_latch = 0;
                self.gun.end_mode();
            }
            State::LightJakGetOn { .. } => {
                self.lightjak.interp = 1.0;
                self.danger_set(None);
                self.restore_gravity();
                self.gun.end_mode();
            }
            State::LightJakGetOff => self.lightjak_end_mode(true),
            State::LightJakSwoop { .. } | State::LightJakSwoopFalling => {
                if !matches!(next, State::LightJakSwoop { .. }) {
                    self.restore_gravity();
                }
                self.target_exit_light();
            }
            _ => {}
        }
    }

    fn restore_gravity(&mut self) {
        self.control.gravity_max = STANDARD_GRAVITY_MAX;
        self.control.gravity_length = STANDARD_GRAVITY;
    }

    fn target_exit_light(&mut self) {
        let c = &mut self.control;
        c.mod_surface = surface::WALK;
        c.draw_offset_y = 0.0;
        c.force_turn_to_strength = 0.0;
        c.bend_target = 0.0;
        self.danger_set(None);
    }

    fn near_ground(&self, height: f32) -> bool {
        let c = &self.control;
        c.height_above_ground() < height
            && matches!(
                c.gspot_pat.mode,
                crate::collide::PatMode::Ground | crate::collide::PatMode::Halfpipe
            )
    }

    pub(crate) fn lightjak_trans(&mut self) -> Option<State> {
        match self.state {
            State::LightJakSwoop { .. } => {
                if self.control.on_surface() {
                    return Some(State::HitGround { stuck: false });
                }
                self.foot.sliding_start_time = self.time;
                let up = self.control.gravity_normal.dot(self.control.transv);
                (up < 0.0 && self.near_ground(4096.0)).then_some(State::LightJakSwoopFalling)
            }
            State::LightJakSwoopFalling => {
                if self.pad.pressed(button::X) && !self.near_ground(8192.0) {
                    return Some(State::LightJakSwoop {
                        first: false,
                        held: 1.0,
                    });
                }
                self.falling_trans(0, false)
            }
            _ => None,
        }
    }

    pub(crate) fn lightjak_code(&mut self) -> Option<State> {
        match self.state {
            State::PowerJakGetOn => self.powerjak_get_on_code(),
            State::LightJakGetOn { swoop } => self.lightjak_get_on_code(swoop),
            State::LightJakGetOff => self.lightjak_get_off_code(),
            State::LightJakSwoop { first: true, .. } => self.swoop_code(),
            State::LightJakSwoop { first: false, .. } => self.swoop_again_code(),
            State::LightJakSwoopFalling => self.swoop_falling_code(),
            _ => None,
        }
    }

    /// The falling animation while waiting to land before a change.
    fn falling_anim_trans(&mut self) {
        let c = self.chan;
        if !c.is_any(&[anim::JUMP_LOOP, anim::JUMP_LAND]) {
            self.ja_push(seconds(0.33));
            self.chan.anim = anim::JUMP_LOOP;
        } else if self.control.on_surface()
            && !c.is(anim::JUMP_LAND)
            && self.control.status & status::ON_WATER == 0
        {
            self.ja_push(seconds(0.02));
            self.chan.anim = anim::JUMP_LAND;
        } else if c.is(anim::JUMP_LOOP) {
            self.chan
                .eval_with(&self.anims, NumFunc::Loop { rate: 1.0 });
        } else {
            self.chan.eval_with(&self.anims, NumFunc::seek(1.0));
        }
    }

    /// Waiting in the air for the ground, at most a second; then a fall.
    fn wait_to_land(&mut self) -> Result<bool, State> {
        if self.control.on_surface() {
            return Ok(true);
        }
        self.falling_anim_trans();
        self.code.count += crate::FRAME_TICKS as i32;
        if i64::from(self.code.count) >= seconds(1.0) {
            return Err(State::Falling { uppercut: false });
        }
        Ok(false)
    }

    fn blending(&self) -> bool {
        self.time < self.push.tick + self.push.ticks
    }

    /// The power button held: what it is asked for, and when it lets go.
    fn powerjak_poll(&mut self, settled: bool) -> Option<State> {
        let light = self.lightjak.on;
        let l2 = self.pad.hold(button::L2);
        if light && !l2 && self.lightjak.get_on_latch == 0 {
            return Some(if self.lightjak.get_off_lock {
                State::Stance
            } else {
                State::LightJakGetOff
            });
        }
        if (!self.want_to_powerjak() || self.pad.pressed(button::R2))
            && self.time_elapsed(self.state_time, seconds(0.05))
        {
            return Some(State::Stance);
        }
        if self.pad.pressed(button::TRIANGLE) {
            self.lightjak.get_off_lock = true;
        }
        let mut latched = false;
        if self.pad.pressed(button::X) {
            self.lightjak.get_off_lock = true;
            if SWOOP_INC <= self.lightjak.eco
                && self.control.current.flags & (flag::NO_ATTACK | flag::NO_FEET) == 0
                && self.want_to_lightjak()
            {
                self.lightjak.get_on_latch = LATCH_SWOOP;
                latched = true;
            }
        }
        if !latched && self.pad.pressed(button::SQUARE) {
            self.lightjak.get_off_lock = true;
        }
        if !latched && self.pad.pressed(button::CIRCLE) {
            self.lightjak.get_off_lock = true;
        }
        if self.blending() || !settled {
            return None;
        }
        if self.lightjak.get_on_latch != 0 && self.want_to_lightjak() {
            if light {
                self.lightjak.swoop = true;
                self.spawn_wings(WingsStart::Open);
                return Some(State::Stance);
            }
            return Some(State::LightJakGetOn { swoop: true });
        }
        if self.lightjak.get_on_latch != 0 {
            self.lightjak.get_on_latch = 0;
        }
        None
    }

    /// Holding the power button: Jak stops and readies the change while it
    /// is held, and changes once a power is asked for.
    fn powerjak_get_on_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.control.mod_surface = surface::foot::LIGHTJAK_TRANS;
                    self.lightjak.get_on_latch = 0;
                    self.gun.end_mode();
                    self.set_forward_vel(0.0);
                    self.code.count = 0;
                    self.goto(1);
                }
                1 => match self.wait_to_land() {
                    Err(fall) => return Some(fall),
                    Ok(true) => self.goto(2),
                    Ok(false) => return self.powerjak_poll(false),
                },
                2 => {
                    self.ja_push(seconds(0.1));
                    self.ja_play(anim::POWERJAK_GET_ON, 1.0);
                    self.goto(3);
                }
                3 | 5 => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            self.goto(4);
                            continue;
                        }
                    }
                    return self.powerjak_poll(true);
                }
                _ => {
                    self.ja_play(anim::POWERJAK_GET_ON_LOOP, 1.0);
                    self.goto(5);
                }
            }
        }
    }

    /// The change into Light Jak: he rises on the change's animation,
    /// comes out of it and lands, the wings spreading if the flight came
    /// with it.
    fn lightjak_get_on_code(&mut self, swoop: bool) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.gun.end_mode();
                    let l = &mut self.lightjak;
                    l.swoop = swoop;
                    l.start_time = self.time;
                    l.latch_out_time = 0;
                    l.on = true;
                    self.events.push(Event::LightJak { on: true });
                    self.control.mod_surface = surface::foot::LIGHTJAK_TRANS;
                    self.start_attack();
                    self.danger_set(Some(crate::Danger::GetOn));
                    self.set_forward_vel(0.0);
                    self.ja_push(seconds(0.1));
                    self.ja_play(anim::LIGHTJAK_GET_ON, 1.0);
                    self.code.x = if swoop { 2.0 } else { 1.0 };
                    self.goto(1);
                }
                1 => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            self.goto(2);
                            continue;
                        }
                    }
                    self.compute_delta_align();
                    self.align(crate::align::opts::Y_VEL, 1.0, self.code.x, 1.0);
                    return None;
                }
                2 => {
                    self.control.transv = glam::Vec3::ZERO;
                    self.lightjak.interp = 1.0;
                    self.restore_gravity();
                    self.chan
                        .set(anim::LIGHTJAK_GET_ON_OUT, NumFunc::seek(1.0), 0.0);
                    self.goto(3);
                }
                3 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    if swoop {
                        self.spawn_wings(WingsStart::Open);
                    }
                    self.chan
                        .set(anim::LIGHTJAK_GET_ON_LAND, NumFunc::seek(1.0), 0.0);
                    self.goto(4);
                }
                _ => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            return Some(State::Stance);
                        }
                    }
                    if self.pad.stick0_speed != 0.0 && 31.0 < self.chan.aframe_num(&self.anims) {
                        return Some(State::Stance);
                    }
                    return None;
                }
            }
        }
    }

    /// The change back: on the ground, the change's animation backwards if
    /// it had only begun, else the change back's own.
    fn lightjak_get_off_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.count = 0;
                    if self.chan.is(anim::LIGHTJAK_GET_ON) {
                        self.goto(2);
                    } else {
                        self.goto(1);
                    }
                }
                1 => match self.wait_to_land() {
                    Err(fall) => return Some(fall),
                    Ok(true) => self.goto(2),
                    Ok(false) => return None,
                },
                2 => {
                    self.control.transv = glam::Vec3::ZERO;
                    self.lightjak_end_mode(true);
                    if self.control.status & status::ON_WATER != 0 {
                        return Some(State::Stance);
                    }
                    if self.chan.is(anim::LIGHTJAK_GET_ON) {
                        self.chan.func = NumFunc::seek_to(0.0, 1.0);
                        self.goto(3);
                    } else {
                        self.ja_push(seconds(0.05));
                        self.ja_play(anim::LIGHTJAK_GET_OFF, 1.0);
                        self.goto(4);
                    }
                }
                3 => {
                    if self.chan.done(&self.anims) {
                        self.ja_push(seconds(0.1));
                        return Some(State::Stance);
                    }
                    if self.code.arrived {
                        let l = &mut self.lightjak;
                        l.interp = seek(l.interp, 0.0, 2.0 * SECONDS_PER_FRAME);
                        return None;
                    }
                    self.chan.eval(&self.anims);
                    self.code.arrived = true;
                }
                _ => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            return Some(State::Stance);
                        }
                    }
                    let aframe = self.chan.aframe_num(&self.anims);
                    self.lightjak.interp = lerp_scale(1.0, 0.0, aframe, 10.0, 60.0);
                    if aframe >= 24.0 && self.pad.stick0_speed != 0.0 && self.lightjak.interp == 0.0
                    {
                        return Some(State::Stance);
                    }
                    return None;
                }
            }
        }
    }

    /// A flap: the vertical speed set to the flap's own, the horizontal
    /// kept.
    fn flap(&mut self) {
        let c = &mut self.control;
        let up = c.gravity_normal.dot(c.transv);
        let flat = c.transv - c.gravity_normal * up;
        c.transv = c.gravity_normal * self.lightjak.swoop_impulse + flat;
    }

    /// The press after half a second in the air: another flap, as strong
    /// as the time since the last allows.
    fn again(&self) -> Option<State> {
        (self.pad.pressed(button::X) && self.time_elapsed(self.state_time, seconds(0.5))).then(
            || State::LightJakSwoop {
                first: false,
                held: (self.foot.sliding_start_time - self.state_time) as f32
                    / crate::TICKS_PER_SECOND as f32,
            },
        )
    }

    fn swoop_code(&mut self) -> Option<State> {
        if self.code.pc == 0 {
            self.ja_push(seconds(0.05));
            self.chan
                .set(anim::LIGHTJAK_SWOOP1, NumFunc::seek(1.0), 0.0);
            self.flap();
            self.goto(1);
            return None;
        }
        if self.code.arrived {
            return None;
        }
        self.chan.eval(&self.anims);
        self.code.arrived = true;
        if let Some(next) = self.again() {
            return Some(next);
        }
        self.chan
            .done(&self.anims)
            .then_some(State::LightJakSwoopFalling)
    }

    fn swoop_again_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    let aframe = self.chan.aframe_num(&self.anims);
                    self.code.x = 44.0;
                    self.code.count = 6;
                    if self.chan.is(anim::LIGHTJAK_SWOOP1) {
                        self.code.x = lerp_scale(48.0, 44.0, aframe, 30.0, 44.0);
                        self.code.count = 30;
                    } else if self.chan.is(anim::LIGHTJAK_SWOOP2) {
                        self.code.x = lerp_scale(48.0, 44.0, aframe, 54.0, 74.0);
                        self.code.count = 30;
                    } else if self
                        .chan
                        .is_any(&[anim::LIGHTJAK_SWOOP_FALL, anim::LIGHTJAK_SWOOP_FALL_LOOP])
                    {
                        self.ja_push(seconds(0.2));
                        let from = self.aframe(anim::LIGHTJAK_SWOOP2, 69.0);
                        self.chan
                            .set(anim::LIGHTJAK_SWOOP2, NumFunc::seek(2.0), from);
                        self.goto(1);
                        continue;
                    }
                    self.goto(2);
                }
                1 => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            self.code.count = 0;
                            self.goto(2);
                            continue;
                        }
                    }
                    let c = &mut self.control;
                    let up = c.gravity_normal.dot(c.transv);
                    let flat = c.transv - c.gravity_normal * up;
                    c.transv = c.gravity_normal * (0.95 * up) + flat;
                    return None;
                }
                2 => {
                    self.ja_push(i64::from(self.code.count));
                    let from = self.aframe(anim::LIGHTJAK_SWOOP2, self.code.x);
                    self.chan
                        .set(anim::LIGHTJAK_SWOOP2, NumFunc::seek(1.0), from);
                    self.goto(3);
                }
                3 => {
                    if self.chan.aframe_num(&self.anims) >= 48.0 {
                        self.flap();
                        self.goto(4);
                        return None;
                    }
                    if self.code.arrived {
                        return None;
                    }
                    self.chan.eval(&self.anims);
                    self.code.arrived = true;
                }
                _ => {
                    if self.code.arrived {
                        return None;
                    }
                    self.chan.eval(&self.anims);
                    self.code.arrived = true;
                    if self.chan.aframe_num(&self.anims) >= 54.0
                        && let Some(next) = self.again()
                    {
                        return Some(next);
                    }
                    return self
                        .chan
                        .done(&self.anims)
                        .then_some(State::LightJakSwoopFalling);
                }
            }
        }
    }

    fn swoop_falling_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.1));
                    self.ja_play(anim::LIGHTJAK_SWOOP_FALL, 1.0);
                    self.goto(1);
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(2);
                }
                2 => {
                    self.ja_play(anim::LIGHTJAK_SWOOP_FALL_LOOP, 1.0);
                    self.goto(3);
                }
                _ => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(2);
                }
            }
        }
    }

    /// Landing out of a flight plays the swoop's landing.
    pub(crate) fn landing_from_swoop(&self) -> bool {
        self.lightjak.on && self.chan.is_any(&SWOOP_ANIMS)
    }

    /// The animation standing plays as Light Jak, and whether it comes in
    /// through the stance change.
    pub(crate) fn light_stance(&self) -> Option<bool> {
        let l = &self.lightjak;
        (l.on && l.latch_out_time == 0).then(|| {
            !self
                .chan
                .is_any(&[anim::LIGHTJAK_SWOOP_LAND, anim::LIGHTJAK_GET_ON_LAND])
        })
    }
}
