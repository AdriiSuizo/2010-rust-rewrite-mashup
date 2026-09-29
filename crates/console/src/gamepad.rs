//! The controller: its sticks as movement and turn rates, and its buttons
//! as menu keys while a menu is up. Its gameplay buttons go through the
//! bind table with the keyboard's.
use bevy::input::ButtonInput;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

/// The sticks after their deadzones, as the stick layout assigns them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Sticks {
    /// Forward and right.
    pub movement: Vec2,
    /// Right and up.
    pub look: Vec2,
}

/// A stick with its centre `deadzone` cut out and the rest rescaled to
/// 0..1, keeping its direction.
fn radial(stick: Vec2, deadzone: f32) -> Vec2 {
    let length = stick.length();
    if length <= deadzone || length <= f32::EPSILON {
        return Vec2::ZERO;
    }
    let scaled = ((length - deadzone) / (1.0 - deadzone).max(0.01)).min(1.0);
    stick / length * scaled
}

pub(crate) fn sticks(pad: &Gamepad, settings: &frame::GameSettings) -> Sticks {
    let left = radial(pad.left_stick(), settings.pad_deadzone_left);
    let right = radial(pad.right_stick(), settings.pad_deadzone_right);
    match settings.pad_stick_layout {
        // Southpaw: the sticks swap.
        1 => Sticks { movement: Vec2::new(right.y, right.x), look: left },
        // Legacy: the left stick moves forward and turns, the right looks
        // up and down and strafes.
        2 => Sticks { movement: Vec2::new(left.y, right.x), look: Vec2::new(left.x, right.y) },
        3 => Sticks { movement: Vec2::new(right.y, left.x), look: Vec2::new(right.x, left.y) },
        _ => Sticks { movement: Vec2::new(left.y, left.x), look: right },
    }
}

/// The response to a look stick's deflection, 0..1.
fn curve(deflection: f32, kind: u8) -> f32 {
    let d = deflection.clamp(0.0, 1.0);
    match kind {
        1 => d,
        2 => 1.0 - (1.0 - d) * (1.0 - d),
        _ => 0.35 * d + 0.65 * d * d * d,
    }
}

/// The look stick through its response curve, right and up, with the look
/// inversion applied to up.
pub(crate) fn shaped_look(look: Vec2, settings: &frame::GameSettings) -> Vec2 {
    let deflection = look.length();
    if deflection <= f32::EPSILON {
        return Vec2::ZERO;
    }
    let shaped = look / deflection * curve(deflection, settings.pad_curve);
    Vec2::new(shaped.x, if settings.pad_invert { -shaped.y } else { shaped.y })
}

/// Follows the controller in use: the last one with a button pressed or a
/// stick pushed well off centre. Connections are logged with their names.
pub(crate) fn track_active_pad(
    gamepads: Query<(Entity, &Gamepad, Option<&Name>)>,
    mut connections: MessageReader<bevy::input::gamepad::GamepadConnectionEvent>,
    mut active: ResMut<frame::ActivePad>,
) {
    for event in connections.read() {
        match &event.connection {
            bevy::input::gamepad::GamepadConnection::Connected { name, .. } => {
                diag::info!(Ui, "controller connected: {name}");
            }
            bevy::input::gamepad::GamepadConnection::Disconnected => {
                diag::info!(Ui, "controller disconnected");
            }
        }
    }
    if active.0.is_some_and(|entity| gamepads.get(entity).is_err()) {
        active.0 = None;
    }
    for (entity, pad, name) in &gamepads {
        let moved = pad.left_stick().length() > 0.5 || pad.right_stick().length() > 0.5;
        if (pad.get_just_pressed().next().is_some() || moved) && active.0 != Some(entity) {
            diag::info!(Ui, "controller in use: {}", name.map_or("unnamed", |n| n.as_str()));
            active.0 = Some(entity);
        }
    }
}

/// Menu directions repeat while held: after the first press, then this
/// often.
const REPEAT_DELAY: f32 = 0.4;
const REPEAT_EVERY: f32 = 0.12;

#[derive(Default)]
pub(crate) struct PadMenuKeys {
    /// Keys held on the controller's behalf.
    held: Vec<KeyCode>,
    /// A direction pulsed last frame, released now.
    pulsed: Option<KeyCode>,
    /// The direction held and when it next repeats.
    direction: Option<(KeyCode, f32)>,
}

/// While a menu is up the controller drives it as the keyboard does: the
/// D-pad or left stick moves, A accepts, B backs out. Start opens and
/// closes the menu anywhere.
pub(crate) fn drive_menus_with_pad(
    gamepads: Query<&Gamepad>,
    active: Res<frame::ActivePad>,
    menu: Res<ui::MenuEnabled>,
    pending: Res<crate::user_settings::PendingMenuBinding>,
    time: Res<Time>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: Local<PadMenuKeys>,
) {
    if let Some(key) = state.pulsed.take() {
        keys.release(key);
    }
    let pad = active.0.and_then(|entity| gamepads.get(entity).ok());
    let capturing = pending.capturing_pad();
    let mut wanted: Vec<KeyCode> = Vec::new();
    if let Some(pad) = pad.filter(|_| !capturing) {
        if pad.pressed(GamepadButton::Start) {
            wanted.push(KeyCode::Escape);
        }
        if menu.0 {
            if pad.pressed(GamepadButton::South) {
                wanted.push(KeyCode::Enter);
            }
            if pad.pressed(GamepadButton::East) {
                wanted.push(KeyCode::Escape);
            }
            let stick = pad.left_stick();
            let direction = if pad.pressed(GamepadButton::DPadUp) || stick.y > 0.6 {
                Some(KeyCode::ArrowUp)
            } else if pad.pressed(GamepadButton::DPadDown) || stick.y < -0.6 {
                Some(KeyCode::ArrowDown)
            } else if pad.pressed(GamepadButton::DPadLeft) || stick.x < -0.6 {
                Some(KeyCode::ArrowLeft)
            } else if pad.pressed(GamepadButton::DPadRight) || stick.x > 0.6 {
                Some(KeyCode::ArrowRight)
            } else {
                None
            };
            let now = time.elapsed_secs();
            let fire = match (direction, state.direction) {
                (Some(key), Some((held, next))) if key == held => {
                    if now >= next {
                        state.direction = Some((key, now + REPEAT_EVERY));
                        true
                    } else {
                        false
                    }
                }
                (Some(key), _) => {
                    state.direction = Some((key, now + REPEAT_DELAY));
                    true
                }
                (None, _) => {
                    state.direction = None;
                    false
                }
            };
            if fire && let Some(key) = direction {
                keys.press(key);
                state.pulsed = Some(key);
            }
        } else {
            state.direction = None;
        }
    }
    wanted.dedup();
    let held = std::mem::take(&mut state.held);
    for key in &held {
        if !wanted.contains(key) {
            keys.release(*key);
        }
    }
    for key in &wanted {
        if !held.contains(key) {
            keys.press(*key);
        }
    }
    state.held = wanted;
}
