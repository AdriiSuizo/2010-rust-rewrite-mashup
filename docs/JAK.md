# Jak Mode

Jak 3's JET-Board and Blaster, rewritten in Rust and played on any MW2 map or
the Minecraft world. Press **K**, or type `jak on` in the console. Nothing
extra is needed: no Jak 3 files, no OpenGOAL, no emulator.

| keyboard | controller | Jak 3 button | action |
|---|---|---|---|
| WASD, mouse | left stick, right stick | stick | move; camera |
| Space | A | X | jump (hold for height), on the board too |
| F or right click | RT | R2 | get on the board; again, a second after it touched ground, to get off |
| left click | RB | R1 | Blaster on foot; flip on the board in the air |
| Ctrl | LB | L1 | duck and charge a higher jump on the board |
| Ctrl or Shift + direction | LB or LT + stick | L1 / L2 | board tricks in the air |
| E | B | circle | board zap |
| K | | | back to MW2 (`jak off`; `jak status` in the console) |

## How it works

| piece | where | what |
|---|---|---|
| gameplay | [`crates/jak_mode`](../crates/jak_mode) | Jak on foot, the board and the Blaster in Jak's own units (4096 a meter, 300 ticks a second) at 60 Hz; no Bevy, no game files |
| collision | `jak_mode::collide` | swept spheres against triangles from the host: map clip collision, or the Minecraft blocks around Jak |
| mode | [`crates/render_anim/src/jak.rs`](../crates/render_anim/src/jak.rs) | toggling, unit and axis conversion (1 m = 25 map units), input, follow camera, shots into blocks and mobs |
| shots | [`render_frontend` fx](../crates/render_frontend/src/adapters/fx/system.rs) | each shot drawn as a tracer beam from its tail to its head |
| parity | [`crates/approved_tests/src/jak_parity.rs`](../crates/approved_tests/src/jak_parity.rs) | `cargo test -p approved_tests jak_parity`: thrust curve, jump heights, hop timing, fire delay, shot flight and impact, sweeps |

The movement is Jak 3's own, read from the OpenGOAL decompilation and rebuilt:
the control pipeline, surfaces and their multipliers, collision reactions, the
board's thrust, suspension, gravity per state, spins, flips, tricks, charge
jump, zap, glancing off walls, get on and off, and the Blaster's draw, queue
and fire delay. The board never stops by itself: released, it still pushes at
0.4 of its thrust, so it cruises at about 10 m/s.

A Blaster shot does 2 of Jak's damage; the world scales it by 12.5 to MW2's
100 health, so a block takes it like a 25-damage bullet.

## Not yet

- The MW2 soldier stands in for Jak and a slab for the board; no Jak model,
  animations or sounds. The camera is a plain follow camera.
- Grinding, halfpipes, the turn-around, wall kicks and jump kicks, being hit.
- Zap and spin damage, shots against MW2 players, the other guns.
