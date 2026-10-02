//! Jak's animation clock. States drive gameplay windows off the frame an
//! animation has reached, so the simulation keeps the same per-frame
//! playback the game does: a frame number advanced by a playback function
//! and the animation's own speed. Only timing lives here; poses are the
//! renderer's.
//!
//! The timing itself is the player's: [`Anims::from_rows`] takes it from
//! their own Jak 3 files at run time. Without them every animation gets
//! [`NOMINAL`].
mod table;

pub use table::*;

/// One animation's timing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimInfo {
    pub frames: u16,
    /// Frames advanced per 60 Hz frame at rate 1.
    pub speed: f32,
    pub artist_base: f32,
    pub artist_step: f32,
}

/// What an animation the player's files do not time plays at.
pub const NOMINAL: AnimInfo = AnimInfo {
    frames: 21,
    speed: 0.5,
    artist_base: 0.0,
    artist_step: 1.0,
};

/// The timing of every animation in [`NAMES`].
#[derive(Clone, Debug, PartialEq)]
pub struct Anims {
    infos: Vec<AnimInfo>,
    /// How many came from the player's files.
    pub timed: usize,
}

impl Default for Anims {
    fn default() -> Self {
        Self::nominal()
    }
}

impl Anims {
    pub fn nominal() -> Self {
        Self {
            infos: vec![NOMINAL; NAMES.len()],
            timed: 0,
        }
    }

    /// Timing by animation name; names Jak's states never play are skipped.
    pub fn from_rows<'a>(rows: impl IntoIterator<Item = (&'a str, AnimInfo)>) -> Self {
        let mut anims = Self::nominal();
        for (name, info) in rows {
            if let Some(anim) = Anim::by_name(name)
                && info.frames > 0
                && info.speed > 0.0
                && info.artist_step != 0.0
            {
                anims.infos[usize::from(anim.0)] = info;
                anims.timed += 1;
            }
        }
        anims
    }

    pub fn info(&self, anim: Anim) -> AnimInfo {
        self.infos
            .get(usize::from(anim.0))
            .copied()
            .unwrap_or(NOMINAL)
    }

    /// The last frame.
    pub fn max(&self, anim: Anim) -> f32 {
        f32::from(self.info(anim).frames.saturating_sub(1))
    }

    /// The frame an artist frame number names.
    pub fn aframe(&self, anim: Anim, artist: f32) -> f32 {
        let i = self.info(anim);
        (artist - i.artist_base) / i.artist_step
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Anim(pub u16);

impl Anim {
    pub fn name(self) -> &'static str {
        NAMES.get(usize::from(self.0)).copied().unwrap_or("")
    }

    pub fn by_name(name: &str) -> Option<Anim> {
        NAMES
            .iter()
            .position(|a| *a == name)
            .map(|i| Anim(i as u16))
    }
}

/// Where a seek heads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Max,
    Frame(f32),
}

/// How a channel's frame moves each time it is evaluated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumFunc {
    /// Stays where it is.
    Identity,
    /// Toward `target` by at most `rate` (times the animation's speed).
    Seek { target: Target, rate: f32 },
    /// Forward by `rate`, wrapping at the end.
    Loop { rate: f32 },
}

impl NumFunc {
    pub const fn seek(rate: f32) -> Self {
        NumFunc::Seek {
            target: Target::Max,
            rate,
        }
    }

    pub const fn seek_to(frame: f32, rate: f32) -> Self {
        NumFunc::Seek {
            target: Target::Frame(frame),
            rate,
        }
    }
}

/// The base animation channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Channel {
    pub anim: Anim,
    pub frame: f32,
    pub func: NumFunc,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            anim: STANCE_LOOP,
            frame: 0.0,
            func: NumFunc::Identity,
        }
    }
}

impl Channel {
    /// Switches to `anim` at `frame` with `func`, without advancing.
    pub fn set(&mut self, anim: Anim, func: NumFunc, frame: f32) {
        self.anim = anim;
        self.func = func;
        self.frame = frame;
    }

    fn target(&self, anims: &Anims, target: Target) -> f32 {
        match target {
            Target::Max => anims.max(self.anim),
            Target::Frame(f) => f,
        }
    }

    /// One evaluation of the playback function.
    pub fn eval(&mut self, anims: &Anims) {
        let speed = anims.info(self.anim).speed;
        match self.func {
            NumFunc::Identity => {}
            NumFunc::Seek { target, rate } => {
                let target = self.target(anims, target);
                self.frame = crate::math::seek(self.frame, target, rate * speed);
            }
            NumFunc::Loop { rate } => {
                let length = anims.max(self.anim);
                if length > 0.0 {
                    let f = self.frame + length + rate * speed;
                    self.frame = f - (f / length).trunc() * length;
                }
            }
        }
    }

    /// Changes the playback function and evaluates it.
    pub fn eval_with(&mut self, anims: &Anims, func: NumFunc) {
        self.func = func;
        self.eval(anims);
    }

    /// A seek that has reached its target; anything else counts as done.
    pub fn done(&self, anims: &Anims) -> bool {
        match self.func {
            NumFunc::Seek { target, .. } => self.frame == self.target(anims, target),
            _ => true,
        }
    }

    /// The current frame in artist numbering.
    pub fn aframe_num(&self, anims: &Anims) -> f32 {
        let i = anims.info(self.anim);
        self.frame * i.artist_step + i.artist_base
    }

    pub fn is(&self, anim: Anim) -> bool {
        self.anim == anim
    }

    pub fn is_any(&self, anims: &[Anim]) -> bool {
        anims.contains(&self.anim)
    }
}

/// When the pose last started blending from the previous one, and over how
/// many ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Push {
    pub tick: i64,
    pub ticks: i64,
}
