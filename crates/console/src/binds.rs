use input_iw4::{command_id_lookup, command_name};

use std::collections::HashMap;

use bevy::input::ButtonInput;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::MouseButton;
use bevy::prelude::Resource;

pub const DEFAULT_CONTROLS: &str = include_str!("../assets/default_controls.cfg");

pub const BINDABLE_KEYS: &[&str] = &[
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "space",
    "tab",
    "shift",
    "ctrl",
    "alt",
    "enter",
    "backspace",
    "escape",
    "uparrow",
    "downarrow",
    "leftarrow",
    "rightarrow",
    "semicolon",
    "quote",
    "comma",
    "period",
    "slash",
    "minus",
    "equal",
    "bracketleft",
    "bracketright",
    "backslash",
    "mouse1",
    "mouse2",
    "mouse3",
    "mouse4",
    "mouse5",
    "BUTTON_A",
    "BUTTON_B",
    "BUTTON_X",
    "BUTTON_Y",
    "BUTTON_LSHLDR",
    "BUTTON_RSHLDR",
    "BUTTON_LTRIG",
    "BUTTON_RTRIG",
    "BUTTON_LSTICK",
    "BUTTON_RSTICK",
    "DPAD_UP",
    "DPAD_DOWN",
    "DPAD_LEFT",
    "DPAD_RIGHT",
    "BUTTON_BACK",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindButton {
    Key(KeyCode),
    Mouse(MouseButton),
    Pad(PadButton),
}

impl BindButton {
    pub fn is_pad(self) -> bool {
        matches!(self, Self::Pad(_))
    }
}

/// A controller button, named as MW2's console builds name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PadButton {
    A,
    B,
    X,
    Y,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Back,
    Start,
}

impl PadButton {
    pub const ALL: [Self; 16] = [
        Self::A,
        Self::B,
        Self::X,
        Self::Y,
        Self::LeftBumper,
        Self::RightBumper,
        Self::LeftTrigger,
        Self::RightTrigger,
        Self::LeftStick,
        Self::RightStick,
        Self::DpadUp,
        Self::DpadDown,
        Self::DpadLeft,
        Self::DpadRight,
        Self::Back,
        Self::Start,
    ];

    pub const fn gamepad_button(self) -> GamepadButton {
        match self {
            Self::A => GamepadButton::South,
            Self::B => GamepadButton::East,
            Self::X => GamepadButton::West,
            Self::Y => GamepadButton::North,
            Self::LeftBumper => GamepadButton::LeftTrigger,
            Self::RightBumper => GamepadButton::RightTrigger,
            Self::LeftTrigger => GamepadButton::LeftTrigger2,
            Self::RightTrigger => GamepadButton::RightTrigger2,
            Self::LeftStick => GamepadButton::LeftThumb,
            Self::RightStick => GamepadButton::RightThumb,
            Self::DpadUp => GamepadButton::DPadUp,
            Self::DpadDown => GamepadButton::DPadDown,
            Self::DpadLeft => GamepadButton::DPadLeft,
            Self::DpadRight => GamepadButton::DPadRight,
            Self::Back => GamepadButton::Select,
            Self::Start => GamepadButton::Start,
        }
    }

    pub fn from_gamepad_button(button: GamepadButton) -> Option<Self> {
        Self::ALL.into_iter().find(|pad| pad.gamepad_button() == button)
    }

    /// The console key name, as `bind` takes it.
    pub const fn console_name(self) -> &'static str {
        match self {
            Self::A => "BUTTON_A",
            Self::B => "BUTTON_B",
            Self::X => "BUTTON_X",
            Self::Y => "BUTTON_Y",
            Self::LeftBumper => "BUTTON_LSHLDR",
            Self::RightBumper => "BUTTON_RSHLDR",
            Self::LeftTrigger => "BUTTON_LTRIG",
            Self::RightTrigger => "BUTTON_RTRIG",
            Self::LeftStick => "BUTTON_LSTICK",
            Self::RightStick => "BUTTON_RSTICK",
            Self::DpadUp => "DPAD_UP",
            Self::DpadDown => "DPAD_DOWN",
            Self::DpadLeft => "DPAD_LEFT",
            Self::DpadRight => "DPAD_RIGHT",
            Self::Back => "BUTTON_BACK",
            Self::Start => "BUTTON_START",
        }
    }

    /// The short label the controls screen shows.
    pub const fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::X => "X",
            Self::Y => "Y",
            Self::LeftBumper => "LB",
            Self::RightBumper => "RB",
            Self::LeftTrigger => "LT",
            Self::RightTrigger => "RT",
            Self::LeftStick => "LS",
            Self::RightStick => "RS",
            Self::DpadUp => "D-UP",
            Self::DpadDown => "D-DOWN",
            Self::DpadLeft => "D-LEFT",
            Self::DpadRight => "D-RIGHT",
            Self::Back => "BACK",
            Self::Start => "START",
        }
    }

    const fn keynum(self) -> usize {
        190 + self as usize
    }
}

/// MW2's console button layouts, as its controls menu offers them.
pub const PAD_LAYOUT_NAMES: [&str; 5] = [
    "Default",
    "Tactical",
    "Lefty",
    "Bumper Jumper",
    "Bumper Jumper Tactical",
];

/// A layout's buttons and the commands they drive.
pub fn pad_layout(layout: usize) -> Vec<(PadButton, &'static str)> {
    use PadButton::*;
    let mut binds = vec![
        (RightTrigger, "+attack"),
        (LeftTrigger, "+speed_throw"),
        (RightBumper, "+frag"),
        (LeftBumper, "+smoke"),
        (A, "+gostand"),
        (B, "+stance"),
        (X, "+usereload"),
        (Y, "weapnext"),
        (LeftStick, "+breath_sprint"),
        (RightStick, "+melee"),
        (DpadUp, "+actionslot 1"),
        (DpadDown, "+actionslot 2"),
        (DpadLeft, "+actionslot 3"),
        (DpadRight, "+actionslot 4"),
        (Back, "+scores"),
    ];
    let mut set = |button: PadButton, command: &'static str| {
        binds.retain(|(b, _)| *b != button);
        binds.push((button, command));
    };
    let tactical = |set: &mut dyn FnMut(PadButton, &'static str)| {
        set(B, "+melee");
        set(RightStick, "+stance");
    };
    match layout {
        1 => tactical(&mut set),
        2 => {
            set(LeftTrigger, "+attack");
            set(RightTrigger, "+speed_throw");
            set(LeftBumper, "+frag");
            set(RightBumper, "+smoke");
        }
        3 | 4 => {
            set(LeftBumper, "+gostand");
            set(A, "+smoke");
            if layout == 4 {
                tactical(&mut set);
            }
        }
        _ => {}
    }
    binds
}

pub struct BindInputs<'a> {
    pub keys: &'a ButtonInput<KeyCode>,
    pub mouse: &'a ButtonInput<MouseButton>,
    pub pad: Option<&'a Gamepad>,
}

impl<'a> BindInputs<'a> {
    pub fn new(keys: &'a ButtonInput<KeyCode>, mouse: &'a ButtonInput<MouseButton>) -> Self {
        Self { keys, mouse, pad: None }
    }

    pub fn with_pad(mut self, pad: Option<&'a Gamepad>) -> Self {
        self.pad = pad;
        self
    }

    pub fn pressed(&self, button: BindButton) -> bool {
        match button {
            BindButton::Key(key) => self.keys.pressed(key),
            BindButton::Mouse(btn) => self.mouse.pressed(btn),
            BindButton::Pad(btn) => self.pad.is_some_and(|pad| pad.pressed(btn.gamepad_button())),
        }
    }

    pub fn just_pressed(&self, button: BindButton) -> bool {
        match button {
            BindButton::Key(key) => self.keys.just_pressed(key),
            BindButton::Mouse(btn) => self.mouse.just_pressed(btn),
            BindButton::Pad(btn) => {
                self.pad.is_some_and(|pad| pad.just_pressed(btn.gamepad_button()))
            }
        }
    }

    pub fn just_released(&self, button: BindButton) -> bool {
        match button {
            BindButton::Key(key) => self.keys.just_released(key),
            BindButton::Mouse(btn) => self.mouse.just_released(btn),
            BindButton::Pad(btn) => {
                self.pad.is_some_and(|pad| pad.just_released(btn.gamepad_button()))
            }
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct KeyBinds {
    map: HashMap<BindButton, u32>,
}

impl KeyBinds {
    pub fn apply_defaults(&mut self) {
        self.map.clear();
        let _ = self.apply_script(DEFAULT_CONTROLS);
        self.apply_pad_layout(0);
    }

    pub fn apply_script(&mut self, script: &str) -> Vec<String> {
        self.apply_script_inner(script, true)
    }

    pub(crate) fn apply_config_script(&mut self, script: &str) -> Vec<String> {
        self.apply_script_inner(script, false)
    }

    fn apply_script_inner(&mut self, script: &str, echo_success: bool) -> Vec<String> {
        let mut output = Vec::new();
        for raw in script.split([';', '\n']) {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let Some(command) = crate::ConsoleCommand::parse(line) else {
                continue;
            };
            match command.name.as_str() {
                "bind" => match self.cmd_bind(&command.args) {
                    Ok(Some(msg)) if echo_success => output.push(msg),
                    Ok(Some(_)) => {}
                    Ok(None) => {}
                    Err(msg) => output.push(msg),
                },
                "unbind" => match self.cmd_unbind(&command.args) {
                    Ok(Some(msg)) if echo_success => output.push(msg),
                    Ok(Some(_)) => {}
                    Ok(None) => {}
                    Err(msg) => output.push(msg),
                },
                "unbindall" => {
                    self.map.clear();
                    if echo_success {
                        output.push("unbindall".into());
                    }
                }
                other => output.push(format!("unknown bind-script command `{other}`")),
            }
        }
        output
    }

    pub fn set(&mut self, button: BindButton, id: u32) {
        self.map.insert(button, id);
    }

    pub fn clear_button(&mut self, button: BindButton) -> bool {
        self.map.remove(&button).is_some()
    }

    pub fn clear_command(&mut self, id: u32) -> bool {
        let before = self.map.len();
        self.map.retain(|_, bound| *bound != id);
        self.map.len() != before
    }

    pub fn clear_all(&mut self) {
        self.map.clear();
    }

    /// Unbinds a command from the keyboard and mouse, or from the
    /// controller, leaving the other's binding alone.
    pub fn clear_command_on(&mut self, id: u32, pad: bool) -> bool {
        let before = self.map.len();
        self.map.retain(|button, bound| *bound != id || button.is_pad() != pad);
        self.map.len() != before
    }

    /// Replaces every controller binding with a console button layout.
    pub fn apply_pad_layout(&mut self, layout: usize) {
        self.map.retain(|button, _| !button.is_pad());
        for (button, command) in pad_layout(layout) {
            if let Some(id) = command_id_lookup(command) {
                self.set(BindButton::Pad(button), id);
            }
        }
    }

    pub fn has_pad_binds(&self) -> bool {
        self.map.keys().any(|button| button.is_pad())
    }

    pub fn get(&self, button: BindButton) -> Option<u32> {
        self.map.get(&button).copied()
    }

    pub fn binding_name(&self, button: BindButton) -> Option<&'static str> {
        self.get(button).and_then(command_name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (BindButton, u32)> + '_ {
        self.map.iter().map(|(b, id)| (*b, *id))
    }

    pub fn list_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .map
            .iter()
            .filter_map(|(button, id)| {
                command_name(*id).map(|name| format!("bind {} {name}", display_button(*button)))
            })
            .collect();
        lines.sort();
        lines.dedup();
        lines
    }

    fn cmd_bind(&mut self, args: &[String]) -> Result<Option<String>, String> {
        match args {
            [] => Ok(None),
            [key] => {
                let buttons =
                    parse_button_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
                let names: Vec<&str> = buttons
                    .iter()
                    .filter_map(|button| self.binding_name(*button))
                    .collect();
                if names.is_empty() {
                    Ok(Some(format!("`{key}` is unbound")))
                } else {
                    Ok(Some(format!("bind {key} {}", names[0])))
                }
            }
            [key, action @ ..] => {
                let action = action.join(" ");
                let id = command_id_lookup(&action)
                    .ok_or_else(|| format!("unknown command `{action}`"))?;
                let buttons =
                    parse_button_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
                for button in buttons {
                    self.set(button, id);
                }
                let name = command_name(id).unwrap_or(action.as_str());
                Ok(Some(format!("bind {key} {name}")))
            }
        }
    }

    fn cmd_unbind(&mut self, args: &[String]) -> Result<Option<String>, String> {
        match args {
            [key] => {
                let buttons =
                    parse_button_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
                let mut any = false;
                for button in buttons {
                    any |= self.clear_button(button);
                }
                if any {
                    Ok(Some(format!("unbind {key}")))
                } else {
                    Ok(Some(format!("`{key}` is unbound")))
                }
            }
            _ => Err("usage: unbind <key>".into()),
        }
    }
}

pub fn host_keynum(button: BindButton) -> usize {
    match button {
        BindButton::Mouse(MouseButton::Left) => 180,
        BindButton::Mouse(MouseButton::Right) => 181,
        BindButton::Mouse(MouseButton::Middle) => 182,
        BindButton::Mouse(MouseButton::Back) => 183,
        BindButton::Mouse(MouseButton::Forward) => 184,
        BindButton::Mouse(_) => 185,
        BindButton::Pad(button) => button.keynum(),
        BindButton::Key(key) => keycode_keynum(key),
    }
}

fn keycode_keynum(key: KeyCode) -> usize {
    match key {
        KeyCode::KeyA => 1,
        KeyCode::KeyB => 2,
        KeyCode::KeyC => 3,
        KeyCode::KeyD => 4,
        KeyCode::KeyE => 5,
        KeyCode::KeyF => 6,
        KeyCode::KeyG => 7,
        KeyCode::KeyH => 8,
        KeyCode::KeyI => 9,
        KeyCode::KeyJ => 10,
        KeyCode::KeyK => 11,
        KeyCode::KeyL => 12,
        KeyCode::KeyM => 13,
        KeyCode::KeyN => 14,
        KeyCode::KeyO => 15,
        KeyCode::KeyP => 16,
        KeyCode::KeyQ => 17,
        KeyCode::KeyR => 18,
        KeyCode::KeyS => 19,
        KeyCode::KeyT => 20,
        KeyCode::KeyU => 21,
        KeyCode::KeyV => 22,
        KeyCode::KeyW => 23,
        KeyCode::KeyX => 24,
        KeyCode::KeyY => 25,
        KeyCode::KeyZ => 26,
        KeyCode::Digit0 => 27,
        KeyCode::Digit1 => 28,
        KeyCode::Digit2 => 29,
        KeyCode::Digit3 => 30,
        KeyCode::Digit4 => 31,
        KeyCode::Digit5 => 32,
        KeyCode::Digit6 => 33,
        KeyCode::Digit7 => 34,
        KeyCode::Digit8 => 35,
        KeyCode::Digit9 => 36,
        KeyCode::Space => 37,
        KeyCode::Tab => 38,
        KeyCode::ShiftLeft => 39,
        KeyCode::ShiftRight => 40,
        KeyCode::ControlLeft => 41,
        KeyCode::ControlRight => 42,
        KeyCode::AltLeft => 43,
        KeyCode::AltRight => 44,
        KeyCode::Enter => 45,
        KeyCode::Backspace => 46,
        KeyCode::Escape => 47,
        KeyCode::ArrowUp => 48,
        KeyCode::ArrowDown => 49,
        KeyCode::ArrowLeft => 50,
        KeyCode::ArrowRight => 51,
        KeyCode::Semicolon => 52,
        KeyCode::Quote => 53,
        KeyCode::Comma => 54,
        KeyCode::Period => 55,
        KeyCode::Slash => 56,
        KeyCode::Minus => 57,
        KeyCode::Equal => 58,
        KeyCode::BracketLeft => 59,
        KeyCode::BracketRight => 60,
        KeyCode::Backslash => 61,
        _ => 62,
    }
}

pub fn parse_button_name(name: &str) -> Option<Vec<BindButton>> {
    let name = name.trim().to_ascii_lowercase();
    let button = match name.as_str() {
        "a" => BindButton::Key(KeyCode::KeyA),
        "b" => BindButton::Key(KeyCode::KeyB),
        "c" => BindButton::Key(KeyCode::KeyC),
        "d" => BindButton::Key(KeyCode::KeyD),
        "e" => BindButton::Key(KeyCode::KeyE),
        "f" => BindButton::Key(KeyCode::KeyF),
        "g" => BindButton::Key(KeyCode::KeyG),
        "h" => BindButton::Key(KeyCode::KeyH),
        "i" => BindButton::Key(KeyCode::KeyI),
        "j" => BindButton::Key(KeyCode::KeyJ),
        "k" => BindButton::Key(KeyCode::KeyK),
        "l" => BindButton::Key(KeyCode::KeyL),
        "m" => BindButton::Key(KeyCode::KeyM),
        "n" => BindButton::Key(KeyCode::KeyN),
        "o" => BindButton::Key(KeyCode::KeyO),
        "p" => BindButton::Key(KeyCode::KeyP),
        "q" => BindButton::Key(KeyCode::KeyQ),
        "r" => BindButton::Key(KeyCode::KeyR),
        "s" => BindButton::Key(KeyCode::KeyS),
        "t" => BindButton::Key(KeyCode::KeyT),
        "u" => BindButton::Key(KeyCode::KeyU),
        "v" => BindButton::Key(KeyCode::KeyV),
        "w" => BindButton::Key(KeyCode::KeyW),
        "x" => BindButton::Key(KeyCode::KeyX),
        "y" => BindButton::Key(KeyCode::KeyY),
        "z" => BindButton::Key(KeyCode::KeyZ),
        "0" => BindButton::Key(KeyCode::Digit0),
        "1" => BindButton::Key(KeyCode::Digit1),
        "2" => BindButton::Key(KeyCode::Digit2),
        "3" => BindButton::Key(KeyCode::Digit3),
        "4" => BindButton::Key(KeyCode::Digit4),
        "5" => BindButton::Key(KeyCode::Digit5),
        "6" => BindButton::Key(KeyCode::Digit6),
        "7" => BindButton::Key(KeyCode::Digit7),
        "8" => BindButton::Key(KeyCode::Digit8),
        "9" => BindButton::Key(KeyCode::Digit9),
        "space" => BindButton::Key(KeyCode::Space),
        "tab" => BindButton::Key(KeyCode::Tab),
        "enter" | "return" => BindButton::Key(KeyCode::Enter),
        "backspace" => BindButton::Key(KeyCode::Backspace),
        "escape" | "esc" => BindButton::Key(KeyCode::Escape),
        "uparrow" | "up" => BindButton::Key(KeyCode::ArrowUp),
        "downarrow" | "down" => BindButton::Key(KeyCode::ArrowDown),
        "leftarrow" | "left" => BindButton::Key(KeyCode::ArrowLeft),
        "rightarrow" | "right" => BindButton::Key(KeyCode::ArrowRight),
        "semicolon" => BindButton::Key(KeyCode::Semicolon),
        "quote" => BindButton::Key(KeyCode::Quote),
        "comma" => BindButton::Key(KeyCode::Comma),
        "period" => BindButton::Key(KeyCode::Period),
        "slash" => BindButton::Key(KeyCode::Slash),
        "minus" => BindButton::Key(KeyCode::Minus),
        "equal" | "equals" => BindButton::Key(KeyCode::Equal),
        "bracketleft" | "[" => BindButton::Key(KeyCode::BracketLeft),
        "bracketright" | "]" => BindButton::Key(KeyCode::BracketRight),
        "backslash" => BindButton::Key(KeyCode::Backslash),
        "shift" | "shiftleft" | "lshift" => {
            return Some(vec![
                BindButton::Key(KeyCode::ShiftLeft),
                BindButton::Key(KeyCode::ShiftRight),
            ]);
        }
        "shiftright" | "rshift" => BindButton::Key(KeyCode::ShiftRight),
        "ctrl" | "control" | "ctrlleft" | "lctrl" => {
            return Some(vec![
                BindButton::Key(KeyCode::ControlLeft),
                BindButton::Key(KeyCode::ControlRight),
            ]);
        }
        "ctrlright" | "rctrl" => BindButton::Key(KeyCode::ControlRight),
        "alt" | "altleft" | "lalt" => {
            return Some(vec![
                BindButton::Key(KeyCode::AltLeft),
                BindButton::Key(KeyCode::AltRight),
            ]);
        }
        "altright" | "ralt" => BindButton::Key(KeyCode::AltRight),
        "mouse1" | "mouseleft" | "lmb" => BindButton::Mouse(MouseButton::Left),
        "mouse2" | "mouseright" | "rmb" => BindButton::Mouse(MouseButton::Right),
        "mouse3" | "mousemiddle" | "mmb" => BindButton::Mouse(MouseButton::Middle),
        "mouse4" => BindButton::Mouse(MouseButton::Back),
        "mouse5" => BindButton::Mouse(MouseButton::Forward),
        other => {
            let upper = other.to_ascii_uppercase();
            return PadButton::ALL
                .into_iter()
                .find(|pad| pad.console_name() == upper || pad.label() == upper)
                .map(|pad| vec![BindButton::Pad(pad)]);
        }
    };
    Some(vec![button])
}

pub fn parse_key_name(name: &str) -> Option<Vec<BindButton>> {
    parse_button_name(name)
}

pub fn display_button(button: BindButton) -> String {
    match button {
        BindButton::Key(key) => display_key(key),
        BindButton::Mouse(MouseButton::Left) => "MOUSE1".into(),
        BindButton::Mouse(MouseButton::Right) => "MOUSE2".into(),
        BindButton::Mouse(MouseButton::Middle) => "MOUSE3".into(),
        BindButton::Mouse(MouseButton::Back) => "MOUSE4".into(),
        BindButton::Mouse(MouseButton::Forward) => "MOUSE5".into(),
        BindButton::Mouse(other) => format!("{other:?}"),
        BindButton::Pad(pad) => pad.console_name().into(),
    }
}

fn display_key(key: KeyCode) -> String {
    match key {
        KeyCode::KeyA => "a",
        KeyCode::KeyB => "b",
        KeyCode::KeyC => "c",
        KeyCode::KeyD => "d",
        KeyCode::KeyE => "e",
        KeyCode::KeyF => "f",
        KeyCode::KeyG => "g",
        KeyCode::KeyH => "h",
        KeyCode::KeyI => "i",
        KeyCode::KeyJ => "j",
        KeyCode::KeyK => "k",
        KeyCode::KeyL => "l",
        KeyCode::KeyM => "m",
        KeyCode::KeyN => "n",
        KeyCode::KeyO => "o",
        KeyCode::KeyP => "p",
        KeyCode::KeyQ => "q",
        KeyCode::KeyR => "r",
        KeyCode::KeyS => "s",
        KeyCode::KeyT => "t",
        KeyCode::KeyU => "u",
        KeyCode::KeyV => "v",
        KeyCode::KeyW => "w",
        KeyCode::KeyX => "x",
        KeyCode::KeyY => "y",
        KeyCode::KeyZ => "z",
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        KeyCode::Space => "SPACE",
        KeyCode::Tab => "TAB",
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "SHIFT",
        KeyCode::ControlLeft | KeyCode::ControlRight => "CTRL",
        KeyCode::AltLeft | KeyCode::AltRight => "ALT",
        KeyCode::Enter => "ENTER",
        KeyCode::Backspace => "BACKSPACE",
        KeyCode::Escape => "ESCAPE",
        KeyCode::ArrowUp => "UPARROW",
        KeyCode::ArrowDown => "DOWNARROW",
        KeyCode::ArrowLeft => "LEFTARROW",
        KeyCode::ArrowRight => "RIGHTARROW",
        KeyCode::Semicolon => "SEMICOLON",
        KeyCode::Quote => "QUOTE",
        KeyCode::Comma => "COMMA",
        KeyCode::Period => "PERIOD",
        KeyCode::Slash => "SLASH",
        KeyCode::Minus => "MINUS",
        KeyCode::Equal => "EQUAL",
        KeyCode::BracketLeft => "BRACKETLEFT",
        KeyCode::BracketRight => "BRACKETRIGHT",
        KeyCode::Backslash => "BACKSLASH",
        other => return format!("{other:?}"),
    }
    .to_owned()
}
