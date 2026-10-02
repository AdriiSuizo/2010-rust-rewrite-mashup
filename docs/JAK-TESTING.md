# Testing Jak Mode locally

Needs: system packages ([`BUILD.md`](BUILD.md)), your **MW2 PC multiplayer**
files, and for the Minecraft map an internet connection and `curl` the first
time. Jak's own model is optional: see [`JAK.md`](JAK.md) for the four GLBs;
without them the soldier and a slab stand in.

```bash
git fetch origin claude/tender-einstein-92k9z2 && git checkout claude/tender-einstein-92k9z2
cargo test -p approved_tests jak_parity world_interact   # no game needed: 46 passed
cp .env.example .env                        # set IW4L_GAMES to the folder holding your MW2 folder
set -a; . ./.env; set +a
make map mp_rust CMDS='spawn assault; force_match_start; jak on'
make map ZONE=minecraft:overworld CMDS='spawn assault; force_match_start; jak on'
```

Without `CMDS`, spawn and press **K** (or `` ` `` then `jak on`). Type
`jak debug` in the console for the state machine's readout: state, the
state before and what changed it, animation, frame and walk mix, the
attack and its live hit spheres and struck targets, the moves the state
takes now (closed ones in brackets), the open cancel window, buffered and
held buttons, velocity, ground or air, the board's latches, Light Jak (eco,
wings, flap speed) and the Use target. On an MW2
map the first start says "building Jak's collision for this map" for a few
seconds. `iw4l-artifacts/logs/latest.log` then shows `Jak collision: N
triangles`, `Jak Mode started`, `Jak: on the board`, and with the GLBs at map
load `Jak model: 289 clips, board true, gun true, 207 animations numbered
from .go art groups` (0 until the `.go` files from [`JAK.md`](JAK.md) are
in the folder).

Local files for the moveset and Light Jak, all from your own extraction
(`decompiler_out/jak3/`), copied into `jak-assets` and never committed:
every `*-ag.go` Jak uses, including `jak-light0-ag.go` and
`jak-ext-light-ag.go` (Light Jak's timing and motion), and
`jakb-wings-wings-lod0.glb` for the wings. The roll, punch lunge, flips
and uppercut carry Jak by the motion in the `.go` files.

| check | do | expect |
|---|---|---|
| mode | K | third-person follow camera, no MW2 gun or reticle, yellow HUD bottom left with state and m/s |
| walk, jump | WASD; Space tapped / held | walks; hops ~0.4 m / ~2.8 m |
| board on | F (or right click, RT) | 0.66 s hop, the board (or slab) under him, state `board-stance`; with the GLBs he crouches on landing and leans into turns |
| cruise | let go of everything | board keeps going at ~9.9 m/s |
| top speed | hold W | climbs smoothly to ~24.5 m/s in about 5 s |
| board jump | Space tapped / held | ~1 m / ~3.5 m |
| charge | hold Ctrl, then Space | duck jump, 2.5 to 5 m |
| air | in the air: LMB + direction; Ctrl or Shift + direction; steer | `board-flip`, flips again while LMB stays down; Ctrl: `board-tricky` (W/S) or `board-trickx` (A/D), Shift: `board-hold`; each plays its whole animation even if you let go; spin, landing a big one boosts |
| jump kick | in the air, Space again | `board-jump-kick`; into a wall: `board-wall-kick` off it |
| climb | Space held; at about 1 m, Alt + Ctrl + D; keeping Ctrl down, let go of Alt and D and hold W; then let go of Ctrl and press it again, quickly and repeatedly | `board-trickx`, then `board-tricky` restarting on each press: Jak keeps climbing. Without Alt the grab never follows the kick |
| wall | ride into a wall at a shallow angle | glances along it, never through |
| off | F again, a second after landing | hops off, state `stance` |
| Blaster | on foot, click LMB | first shot 0.1 s later, then one per click at most every ~0.33 s; yellow beam up to 16 m, gone after 3 s |
| blocks | Minecraft map, shoot a block | cracks and breaks like 25-damage bullets; harder blocks need more |
| mobs | shoot a mob in front | auto-aims at it within 70 m, it takes damage |
| punch, combos | C; C then Space | lunging punch ~9 m; Space while it still carries him: uppercut, then E in the air: air spin |
| spin | E; E then Space | spin kick, Space cuts it into a jump; in the air E spins |
| dive | Space, then C in the air | dive; hold Space as it lands: 7 m flop jump |
| crouch, high jump | hold Ctrl, Space | 7 m high jump; Ctrl walking crawls; C from crouch: uppercut |
| roll | run, Ctrl, Space during or just after | roll, then roll-flip ~16 m |
| double jump | Space twice near the top | double jump |
| skid | run, pull S | turn-around skid, then off the other way |
| hard landing | drop over 30 m | painful landing, damage to the soldier |
| strikes | punch or spin a mob or block | mob takes damage once per attack; blocks crack |
| Light Jak | hold Shift (LT), press Space | change into Light Jak with wings; HUD shows LIGHT JAK and the eco |
| flight | as Light Jak: Space, then Space again near the top | wings spread, ~26 m/s climb, costs 1 eco; Space again after 0.5 s or while falling flaps again; lands with the swoop landing |
| change back | tap Shift (LT) | Light Jak ends, wings fold away |
| crafting table | as the MW2 soldier, look at one and press Use, or knife it | the 3x3 crafting window opens |
| exit | K or `jak off` | back to the MW2 soldier where Jak stood |

Known limitations: no sounds or particles; no edge grab, poles or
swimming; no knockback when Jak is hit; Light Jak has only the flight (no
shield, freeze or healing) and no glow; the folded wings play their
animation without the game's wing physics; grinding and hits on MW2
players are not there. K says "leave Skate first" while skating; E and Q do nothing in
the Minecraft world while Jak Mode is on.
