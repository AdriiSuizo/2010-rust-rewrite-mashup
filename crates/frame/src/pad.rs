use bevy::prelude::{Entity, Resource};

/// The controller the player is using: the last one pressed or moved. A
/// virtual joystick or an idle second pad stays connected without taking
/// over.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivePad(pub Option<Entity>);
