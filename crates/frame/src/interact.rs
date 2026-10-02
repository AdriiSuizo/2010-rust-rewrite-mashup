//! Use on whatever the player looks at, whatever they play as. The world
//! that owns a thing publishes it as the target; the input layer turns the
//! Use key on a target into `pressed` instead of MW2's own use, and the
//! world takes it. With no target, Use stays MW2's. A knife that strikes
//! such a thing uses it too.
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractKind {
    CraftingTable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InteractTarget {
    pub kind: InteractKind,
    /// What the HUD calls it.
    pub label: String,
    /// The block, in the world's own coordinates.
    pub block: [i32; 3],
}

#[derive(Resource, Default, Debug)]
pub struct WorldInteract {
    pub target: Option<InteractTarget>,
    /// Use was pressed on the target since the world last looked.
    pub pressed: bool,
}

impl WorldInteract {
    /// Whether the Use key acts on the world now rather than on MW2.
    pub fn claims_use(&self) -> bool {
        self.target.is_some()
    }

    /// A press of Use: taken when there is a target. Returns whether it
    /// was, so MW2 does not also see it.
    pub fn press_use(&mut self) -> bool {
        if self.claims_use() {
            self.pressed = true;
        }
        self.pressed
    }

    /// What to use this frame: the target, if Use was pressed on it, else
    /// the first thing a knife struck. Clears the press.
    pub fn take(&mut self, knifed: &[InteractTarget]) -> Option<InteractTarget> {
        let pressed = std::mem::take(&mut self.pressed);
        pressed
            .then(|| self.target.clone())
            .flatten()
            .or_else(|| knifed.first().cloned())
    }
}
