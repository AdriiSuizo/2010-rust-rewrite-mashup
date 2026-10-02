//! Jak 3's gameplay, rebuilt in Rust: Jak on foot, the JET-Board and the
//! Blaster, with their own collision against whatever world the host hands
//! over.
//!
//! The simulation keeps Jak's own units (4096 per meter, y up, 300 ticks a
//! second) and runs at the 60 Hz the game's movement was tuned at; the host
//! converts at the boundary and feeds one [`Jak::step`] per frame.
pub mod anim;
pub mod board;
pub mod board_anim;
mod board_code;
pub mod collide;
pub mod control;
pub mod gun;
pub mod math;
pub mod pad;
pub mod projectile;
pub mod surface;
mod target;

use std::sync::Arc;

pub use glam;
use glam::{Quat, Vec3};

pub use anim::{Anim, Anims, Channel, Push};
pub use board::BoardInfo;
pub use board_anim::BoardAnim;
pub use collide::{
    CollideCache, CollideWorld, EmptyWorld, Pat, PatMaterial, PatMode, Tri, TriangleGrid,
};
pub use control::Control;
pub use gun::Gun;
pub use math::{Basis, FRAME_TICKS, METER, SECONDS_PER_FRAME, TICKS_PER_SECOND};
pub use pad::{Pad, PadInput, button};
pub use projectile::{ActorWorld, Projectile, ProjectileHit};

/// What Jak is doing. On-foot states first, board states after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Stance,
    Walk,
    Jump {
        min: f32,
        max: f32,
    },
    DoubleJump {
        min: f32,
        max: f32,
    },
    Falling,
    HitGround,
    BoardGetOn,
    BoardStance,
    BoardDuckStance,
    BoardJump {
        min: f32,
        max: f32,
        duck: bool,
    },
    BoardFalling,
    BoardHitGround,
    BoardTurnTo {
        dir: Vec3,
        duration: i64,
    },
    BoardFlip,
    /// L1 with the stick forward or back: a grab or a nose flip.
    BoardTricky,
    /// L1 with the stick across: a kick flip or a board spin.
    BoardTrickx,
    /// L2: a held grab.
    BoardHold,
    BoardJumpKick,
    BoardWallKick {
        dir: Vec3,
        speed: f32,
    },
    BoardGetOff,
}

impl State {
    pub fn is_board(&self) -> bool {
        !matches!(
            self,
            State::Stance
                | State::Walk
                | State::Jump { .. }
                | State::DoubleJump { .. }
                | State::Falling
                | State::HitGround
        )
    }

    pub(crate) fn kind(&self) -> board::StateKind {
        use board::StateKind as K;
        match self {
            State::BoardGetOn => K::BoardGetOn,
            State::BoardStance => K::BoardStance,
            State::BoardDuckStance => K::BoardDuckStance,
            State::BoardJump { .. } => K::BoardJump,
            State::BoardFalling => K::BoardFalling,
            State::BoardHitGround => K::BoardHitGround,
            State::BoardTurnTo { .. } => K::BoardTurnTo,
            State::BoardFlip => K::BoardFlip,
            State::BoardTricky => K::BoardTricky,
            State::BoardTrickx => K::BoardTrickx,
            State::BoardHold => K::BoardHold,
            State::BoardJumpKick => K::BoardJumpKick,
            State::BoardWallKick { .. } => K::BoardWallKick,
            State::BoardGetOff => K::BoardGetOff,
            _ => K::OnFoot,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            State::Stance => "stance",
            State::Walk => "walk",
            State::Jump { .. } => "jump",
            State::DoubleJump { .. } => "double-jump",
            State::Falling => "falling",
            State::HitGround => "hit-ground",
            State::BoardGetOn => "board-get-on",
            State::BoardStance => "board-stance",
            State::BoardDuckStance => "board-duck-stance",
            State::BoardJump { .. } => "board-jump",
            State::BoardFalling => "board-falling",
            State::BoardHitGround => "board-hit-ground",
            State::BoardTurnTo { .. } => "board-turn-to",
            State::BoardFlip => "board-flip",
            State::BoardTricky => "board-tricky",
            State::BoardTrickx => "board-trickx",
            State::BoardHold => "board-hold",
            State::BoardJumpKick => "board-jump-kick",
            State::BoardWallKick { .. } => "board-wall-kick",
            State::BoardGetOff => "board-get-off",
        }
    }
}

/// Board tricks, as the combo counts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trick {
    Spin,
    Boost,
    Flip,
    Jump,
    DuckJump,
    QuickJump,
    Nosegrab,
    Noseflip,
    Kickspin,
    Kickflip,
    BoardSpin,
    Method,
    MethodCross,
    Backgrab,
    Airwalk,
}

/// Things the host presents: sounds, effects, attacks on the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    BoardOn,
    BoardOff,
    BoardJump,
    BoardLaunch,
    BoardFlip,
    BoardBoost,
    BoardGlance,
    BoardBounce {
        pitch: f32,
    },
    BoardZap {
        center: Vec3,
        radius: f32,
    },
    Trick {
        trick: Trick,
        points: f32,
    },
    Jump,
    Land,
    /// The Blaster fired: muzzle and direction.
    Fire {
        from: Vec3,
        dir: Vec3,
    },
    /// A shot stopped: where, the surface it struck and the damage it does.
    Impact(ProjectileHit),
}

/// Where a state's code is: the step it resumes at and the locals it keeps
/// from one frame to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Code {
    pub pc: u16,
    pub count: i32,
    /// The stick as the state read it when it began.
    pub x: f32,
    pub z: f32,
    pub rate: f32,
    pub flag: bool,
    /// Reached this frame, without a suspend since.
    pub(crate) arrived: bool,
}

/// What changed Jak's state last, and the jump it last set off, for the
/// debug readout and the parity tests.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Trace {
    pub prev: &'static str,
    /// What made the change: the state's per-frame checks ("trans"), its
    /// own sequence ("code"), or the movement ("post").
    pub why: &'static str,
    pub changed_at: i64,
    /// The upward speed the last jump or trick set, units a second.
    pub impulse: f32,
    pub impulse_at: i64,
    /// The highest point since last on the ground.
    pub peak: f32,
}

/// Jak, his board and his gun.
pub struct Jak {
    pub control: Control,
    pub board: BoardInfo,
    pub gun: Gun,
    pub pad: Pad,
    pub state: State,
    pub state_time: i64,
    /// The game clock, in ticks.
    pub time: i64,
    /// The camera's frame: the stick is read through it.
    pub camera: Basis,
    pub projectiles: Vec<Projectile>,
    pub(crate) events: Vec<Event>,
    pub(crate) cache: CollideCache,
    pub(crate) probe_cache: CollideCache,
    pub(crate) pending: Option<State>,
    /// The animation the state is playing, and the timing it plays by.
    pub chan: Channel,
    pub push: Push,
    pub anims: Arc<Anims>,
    pub code: Code,
    pub trace: Trace,
}

impl Jak {
    /// Jak standing at `trans` facing `yaw` (rotation units, 0 along +z).
    pub fn new(trans: Vec3, yaw: f32) -> Self {
        let time = 0;
        Self {
            control: Control::new(trans, yaw, time),
            board: BoardInfo::default(),
            gun: Gun::default(),
            pad: Pad::default(),
            state: State::Falling,
            state_time: time,
            time,
            camera: Basis::IDENTITY,
            projectiles: Vec::new(),
            events: Vec::new(),
            cache: CollideCache::default(),
            probe_cache: CollideCache::default(),
            pending: None,
            chan: Channel::default(),
            push: Push::default(),
            anims: Arc::new(Anims::nominal()),
            code: Code::default(),
            trace: Trace::default(),
        }
    }

    pub fn trans(&self) -> Vec3 {
        self.control.trans
    }

    pub fn velocity(&self) -> Vec3 {
        self.control.transv
    }

    pub fn orientation(&self) -> Quat {
        self.control.quat
    }

    /// The events since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Mount the board at once, without the button, as if R2 had been held
    /// with room above.
    pub fn request_board(&mut self) {
        self.board.latch = true;
    }

    /// One 60 Hz frame: input, the state's checks and its own sequence,
    /// the movement, then the shots in flight.
    pub fn step(
        &mut self,
        input: &PadInput,
        world: &mut dyn CollideWorld,
        actors: Option<&mut dyn ActorWorld>,
    ) {
        self.time += FRAME_TICKS;
        self.pad.update(input);
        self.code.arrived = false;
        self.run_state(world);
        self.post(world);
        if let Some(next) = self.pending.take() {
            self.trace.why = "post";
            self.go(next);
        }
        self.track_peak();
        self.gun_frame(world);
        self.step_projectiles_with(world, actors);
    }

    /// The state's checks, then its code up to its next suspend. A change of
    /// state runs the new state's checks and code in the same frame.
    fn run_state(&mut self, world: &mut dyn CollideWorld) {
        for _ in 0..8 {
            if let Some(next) = self.decide(world) {
                self.trace.why = "trans";
                self.go(next);
                continue;
            }
            match self.run_code() {
                Some(next) => {
                    self.trace.why = "code";
                    self.go(next);
                }
                None => return,
            }
        }
    }

    fn run_code(&mut self) -> Option<State> {
        if self.state.is_board() {
            self.board_code()
        } else {
            None
        }
    }

    fn track_peak(&mut self) {
        let y = self.control.trans.y;
        if self.control.on_surface() {
            self.trace.peak = y;
        } else {
            self.trace.peak = self.trace.peak.max(y);
        }
    }

    /// Leaves the current state for `next`: the old state's exit runs with
    /// the new one already known, then the new one's entry, and its code
    /// starts from the top.
    pub(crate) fn go(&mut self, next: State) {
        let old = self.state;
        if old.is_board() {
            self.board_exit_state(&old, &next);
        } else {
            self.foot_exit(&old, &next);
        }
        self.trace.prev = old.name();
        self.trace.changed_at = self.time;
        self.state = next;
        self.state_time = self.time;
        self.code = Code {
            arrived: true,
            ..Code::default()
        };
        if next.is_board() {
            self.board_enter(&next);
        } else {
            self.foot_enter(&next);
        }
    }

    fn decide(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        if self.state.is_board() {
            self.board_trans()
        } else {
            self.foot_trans(world)
        }
    }

    fn post(&mut self, world: &mut dyn CollideWorld) {
        if self.state.is_board() && self.state != State::BoardGetOff {
            self.board_post(world);
        } else {
            self.target_post(world);
        }
    }
}
