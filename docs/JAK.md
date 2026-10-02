# Jak Mode

Jak 3's JET-Board and Blaster, rewritten in Rust and played on any MW2 map or
the Minecraft world. Press **K**, or type `jak on` in the console. It plays
without Jak 3 files: the MW2 soldier and a slab stand in for Jak and his board.

| keyboard | controller | Jak 3 button | action |
|---|---|---|---|
| WASD, mouse | left stick, right stick | stick | move; camera |
| Space | A | X | jump (hold for height), on the board too |
| F or right click | RT | R2 | get on the board; again, a second after it touched ground, to get off |
| left click | RB | R1 | Blaster on foot; flip on the board in the air |
| Ctrl | LB | L1 | duck and charge a higher jump on the board |
| Ctrl or Shift + direction | LB or LT + stick | L1 / L2 | board tricks in the air |
| Alt + direction | a light push of the stick | stick part way | the partial tilts some trick chains need |
| E | B | circle | board zap |
| K | | | back to MW2 (`jak off`; `jak status` in the console) |
| `jak debug` | | | the state machine's readout under the HUD |

## How it works

| piece | where | what |
|---|---|---|
| gameplay | [`crates/jak_mode`](../crates/jak_mode) | Jak on foot, the board and the Blaster in Jak's own units (4096 a meter, 300 ticks a second) at 60 Hz; no Bevy, no game files |
| collision | `jak_mode::collide` | swept spheres against triangles from the host: map clip collision, or the Minecraft blocks around Jak |
| mode | [`crates/render_anim/src/jak.rs`](../crates/render_anim/src/jak.rs) | toggling, unit and axis conversion (1 m = 25 map units), input, follow camera, shots into blocks and mobs |
| shots | [`render_frontend` fx](../crates/render_frontend/src/adapters/fx/system.rs) | each shot drawn as a tracer beam from its tail to its head |
| model | [`assets::jak_model`](../crates/assets/src/jak_model.rs), [`jak_pose.rs`](../crates/render_anim/src/jak_pose.rs) | Jak, board and gun from the player's GLBs; each state plays Jak 3's animation, the board stance mixes turn, lean and duck as the board drives them |
| states | [`board.rs`](../crates/jak_mode/src/board.rs), [`board_code.rs`](../crates/jak_mode/src/board_code.rs) | each frame a state's checks run, then its own sequence resumes where it waited, then the movement; a change of state runs the new state's checks and sequence in the same frame |
| animation clock | [`jak_mode::anim`](../crates/jak_mode/src/anim/mod.rs) | the frame each state's animation has reached, advanced by its playback function and speed; tricks, flips and kicks last as long as their animations |
| parity | [`crates/approved_tests/src/jak_parity.rs`](../crates/approved_tests/src/jak_parity.rs) | `cargo test -p approved_tests jak_parity`: thrust curve, jump heights, hop timing, fire delay, shot flight and impact, sweeps, trick timing, cancels and chained climbs |

The movement is Jak 3's own, read from the OpenGOAL decompilation and rebuilt:
the control pipeline, surfaces and their multipliers, collision reactions, the
board's thrust, suspension, gravity per state, spins, flips, tricks, charge
jump, zap, glancing off walls, get on and off, and the Blaster's draw, queue
and fire delay. The board never stops by itself: released, it still pushes at
0.4 of its thrust, so it cruises at about 10 m/s.

A Blaster shot does 2 of Jak's damage, scaled by 12.5 to MW2's 100 health.

## Tricks, and why chained tricks fly

Each air trick starts a small jump of its own (0.9 to 1.2 m) on top of what
is left of the current rise, and gravity in a trick is lighter. A trick
plays its animation through; only between animations does it check whether
its button is still held. One that plays out stops the rise, one cancelled
into another trick does not. The checks that start tricks run every frame of
a trick too, so a new one can cut in:

- L1 with the stick forward or back starts a grab, once per L1 press; L1
  with the stick across starts a kick trick, once per landing.
- Leaving a kick or grab clears the grab's latch, so after a kick trick each
  fresh L1 press restarts the grab, never letting it finish.
- The grab only follows the kick when the kick's sideways push was under
  half: the stick part way across (Alt on the keyboard).

So: jump, then Alt + Ctrl + A or D for a kick trick, then W and tap Ctrl
again and again while still rising. Every grab launches faster than the last
and Jak keeps climbing. Nothing in the code asks for this; it comes out of
the rules above, and `jak_parity` checks that it does (and that a full
sideways push does not).

## Jak's own model

Export it from your own Jak 3 with OpenGOAL: in `jak3_config.jsonc` set
`"rip_levels": true`, run Decompile, then copy from
`decompiler_out/jak3/levels/` into a `jak-assets` folder beside `iw4l.exe`
(or name the folder in `.env` as `IW4L_JAK_ASSETS=`): `jakb-normal-lod0.glb`
(Jak), `jakb-lod0.glb` (his animations), `board-lod0.glb`, `gun-lod0.glb`.

The animations' timing comes from your files too. The GLB gives each one's
frames and speed; the art groups give the frame numbering the states time
their windows by. Copy these from `decompiler_out/jak3/raw_obj/` (written by
the same Decompile, `"dump_objs": true`) into the same folder: `jakb-ag.go`,
`jak-ext-board-ag.go`, `jak-board0-ag.go`, `jak-gun0-ag.go`,
`jak-ext-normal-ag.go`. The log then says `207 animations numbered from .go
art groups`. Without them every animation plays a nominal 21 frames at
half speed.

None of these files, nor anything read from them, is part of this
repository.

## Not yet

Sounds, the gun's own fire animation, Jak 3's camera (a follow camera for
now), grinding, halfpipes, the turn-around, being hit, zap and spin damage,
shots against MW2 players, the other guns, Jak's moves on foot (punch, spin,
roll, uppercut, dive, crouch: next), the wall kick's turn from its
animation.
