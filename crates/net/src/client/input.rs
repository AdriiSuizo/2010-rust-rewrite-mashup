use bevy::prelude::Resource;
use input_iw4::{
    ANGLE2SHORT, AdjustAnglesInput, CL_ANGLESPEEDKEY_DEFAULT, CL_PITCHSPEED_DEFAULT,
    CL_YAWSPEED_DEFAULT, ClientInput, CreateCmdInput, axis_to_move, cl_adjust_angles, create_cmd,
    mouse_move_angles, sample_move,
};
use playerstate_iw4::UserCmd;
use playerstate_iw4::buttons;
use std::collections::BTreeSet;

pub const KEY_FRAME_MSEC_MAX: u32 = 200;

pub fn com_frame_time_msec(elapsed_secs: f32) -> i32 {
    (elapsed_secs * 1000.0).max(1.0) as i32
}

pub fn key_frame_msec(delta_secs: f32) -> u32 {
    let ms = (delta_secs.max(0.0) * 1000.0).round() as i32;
    ms.clamp(1, KEY_FRAME_MSEC_MAX as i32) as u32
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct LookState {
    pub angles: [i32; 3],
}

#[derive(Resource, Clone, Debug)]
pub struct ClientActionInput {
    pub client: ClientInput,

    pub scripted_ids: BTreeSet<u32>,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub sensitivity: f32,
    pub mouse_accel: f32,
    pub fov_scale: f32,

    pub shellshock_look_scale: f32,

    pub cgame_max_pitch_speed: f32,

    pub cgame_max_yaw_speed: f32,
    pub m_yaw: f32,
    pub m_pitch: f32,

    pub cl_yawspeed: f32,
    pub now_msec: i32,
    pub frame_msec: u32,

    /// The controller's move stick, forward and right, -1 to 1.
    pub pad_move: [f32; 2],
    /// The controller's look stick as turn rates, pitch and yaw, degrees a
    /// second.
    pub pad_look_rate: [f32; 2],
    /// This frame's controller turn, pitch and yaw, degrees: the rates
    /// over the frame, after aim assist.
    pub pad_look_delta: [f32; 2],
    /// Aim assist: 0 off, 1 slowdown, 2 slowdown and aim snap.
    pub pad_aim_assist: u8,
    /// An aim snap under way: the yaw and pitch still to turn, and the
    /// seconds left to turn them in.
    pub pad_snap: Option<(f32, f32, f32)>,
    /// Aiming down the sight last frame, for the snap on raising it.
    pub pad_was_ads: bool,
}

impl Default for ClientActionInput {
    fn default() -> Self {
        Self {
            client: ClientInput::default(),
            scripted_ids: BTreeSet::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            sensitivity: 5.0,
            mouse_accel: 0.0,
            fov_scale: 1.0,
            shellshock_look_scale: 1.0,
            cgame_max_pitch_speed: 0.0,
            cgame_max_yaw_speed: 0.0,
            m_yaw: 0.022,
            m_pitch: 0.022,
            cl_yawspeed: CL_YAWSPEED_DEFAULT,
            now_msec: 16,
            frame_msec: 16,
            pad_move: [0.0; 2],
            pad_look_rate: [0.0; 2],
            pad_look_delta: [0.0; 2],
            pad_aim_assist: 0,
            pad_snap: None,
            pad_was_ads: false,
        }
    }
}

impl ClientActionInput {
    pub fn consume_edges(&mut self) {
        self.client.kb.clear_was_pressed();
    }
}

pub fn look_angles_from_degrees(viewangles: [f32; 3]) -> [i32; 3] {
    [
        (viewangles[0] * ANGLE2SHORT) as i32,
        (viewangles[1] * ANGLE2SHORT) as i32,
        (viewangles[2] * ANGLE2SHORT) as i32,
    ]
}

pub fn accumulate_look(
    dt: f32,
    now_msec: i32,
    frame_msec: u32,
    client: &mut ClientInput,
    look: &mut LookState,
    cl_yawspeed: f32,
    frozen: bool,
    cgame_max_pitch_speed: f32,
    cgame_max_yaw_speed: f32,
) {
    let (pitch_deg, yaw_deg) = cl_adjust_angles(
        &mut client.kb,
        AdjustAnglesInput {
            dt,
            now_msec,
            frame_msec,
            cl_yawspeed,
            cl_pitchspeed: CL_PITCHSPEED_DEFAULT,
            cl_anglespeedkey: CL_ANGLESPEEDKEY_DEFAULT,
            cgame_max_yaw_speed,
            cgame_max_pitch_speed,
            frozen,
        },
    );
    look.angles[0] = look.angles[0].wrapping_add((pitch_deg * ANGLE2SHORT) as i32);
    look.angles[1] = look.angles[1].wrapping_add((yaw_deg * ANGLE2SHORT) as i32);
}

pub fn build_usercmd(input: &mut ClientActionInput, look: &LookState, server_time: i32) -> UserCmd {
    let now = input.now_msec.max(1);
    let frame = input.frame_msec.max(1);
    let (bits, axes) = sample_move(&mut input.client, now, frame);
    let bits = if input.client.offhand_hold_cancel {
        input.client.offhand_hold_cancel = false;
        bits | buttons::OFFHAND_HOLD_CANCEL
    } else {
        bits
    };

    let mouse_counts = input.mouse_x.abs() + input.mouse_y.abs();
    let frame_f = frame as f32;
    let (mx, my) = input_iw4::apply_mouse_sensitivity(
        input.mouse_x,
        input.mouse_y,
        mouse_counts,
        frame_f,
        input.sensitivity,
        input.mouse_accel,
        input.fov_scale,
    );
    let (mouse_pitch, mouse_yaw) = mouse_move_angles(mx, my, input.m_yaw, input.m_pitch);
    // The controller's sticks add to the keys and the mouse.
    let pad_pitch = (input.pad_look_delta[0] * ANGLE2SHORT) as i32;
    let pad_yaw = (input.pad_look_delta[1] * ANGLE2SHORT) as i32;
    let forward = (axes.forward + input.pad_move[0]).clamp(-1.0, 1.0);
    let right = (axes.right + input.pad_move[1]).clamp(-1.0, 1.0);

    create_cmd(&CreateCmdInput {
        server_time,
        angles: look.angles,
        buttons: bits,
        forwardmove: axis_to_move(forward),
        rightmove: axis_to_move(right),
        mouse_pitch_delta: mouse_pitch + pad_pitch,
        mouse_yaw_delta: mouse_yaw + pad_yaw,
        key_pitch_delta: 0,
        key_yaw_delta: 0,
        frozen: false,
    })
}

/// Aim assist for a controller: over a target the turn slows; raising the
/// sight near one pulls the aim onto it (`mode` 2). `targets` are map
/// points with a radius; `eye` and `angles` (pitch, yaw) are the view.
pub fn pad_aim_assist(
    input: &mut ClientActionInput,
    eye: [f32; 3],
    angles: [f32; 2],
    targets: &[([f32; 3], f32)],
    ads: bool,
    dt: f32,
) {
    const SLOWDOWN_HIP: f32 = 0.55;
    const SLOWDOWN_ADS: f32 = 0.4;
    const PADDING_DEG: f32 = 1.5;
    const SNAP_CONE_DEG: f32 = 7.0;
    const SNAP_SECONDS: f32 = 0.12;
    const RANGE: f32 = 4000.0;

    let mut delta = [input.pad_look_rate[0] * dt, input.pad_look_rate[1] * dt];
    let mode = input.pad_aim_assist;
    // The closest target to the crosshair, by angle: its offset in yaw and
    // pitch, and how far inside its padded size the crosshair is.
    let best = (mode > 0)
        .then(|| {
            targets
                .iter()
                .filter_map(|&(point, radius)| {
                    let d = [point[0] - eye[0], point[1] - eye[1], point[2] - eye[2]];
                    let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    if !(1.0..RANGE).contains(&dist) {
                        return None;
                    }
                    let yaw = d[1].atan2(d[0]).to_degrees();
                    let pitch = -(d[2] / dist).asin().to_degrees();
                    let dyaw = (yaw - angles[1] + 540.0).rem_euclid(360.0) - 180.0;
                    let dpitch = pitch - angles[0];
                    let off = (dyaw * dyaw + dpitch * dpitch).sqrt();
                    let size = (radius / dist).atan().to_degrees() + PADDING_DEG;
                    Some((dyaw, dpitch, off, size))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2))
        })
        .flatten();
    if let Some((_, _, off, size)) = best
        && off <= size
    {
        let slow = if ads { SLOWDOWN_ADS } else { SLOWDOWN_HIP };
        delta[0] *= slow;
        delta[1] *= slow;
    }
    if mode == 2
        && ads
        && !input.pad_was_ads
        && let Some((dyaw, dpitch, off, _)) = best
        && off <= SNAP_CONE_DEG
    {
        input.pad_snap = Some((dyaw, dpitch, SNAP_SECONDS));
    }
    if !ads {
        input.pad_snap = None;
    }
    if let Some((yaw_left, pitch_left, seconds)) = input.pad_snap.as_mut() {
        let part = (dt / (*seconds).max(dt)).min(1.0);
        delta[1] += *yaw_left * part;
        delta[0] += *pitch_left * part;
        *yaw_left *= 1.0 - part;
        *pitch_left *= 1.0 - part;
        *seconds -= dt;
        if *seconds <= 0.0 {
            input.pad_snap = None;
        }
    }
    input.pad_was_ads = ads;
    input.pad_look_delta = delta;
}

pub fn remote_control_axes(input: &ClientActionInput, mouse_x: f32, mouse_y: f32) -> [u8; 2] {
    let (mx, my) = input_iw4::apply_mouse_sensitivity(
        mouse_x,
        mouse_y,
        mouse_x.abs() + mouse_y.abs(),
        input.frame_msec.max(1) as f32,
        input.sensitivity,
        input.mouse_accel,
        input.fov_scale,
    );
    let axis = |v: f32| (v.clamp(-1.0, 1.0) * 127.0).round() as i8 as u8;
    [axis(input.m_pitch * my), axis(-input.m_yaw * mx)]
}

pub fn idle_usercmd(server_time: i32) -> UserCmd {
    UserCmd {
        server_time,
        ..UserCmd::default()
    }
}
