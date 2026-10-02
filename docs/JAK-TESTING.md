# Testing Jak Mode locally

Needs: system packages ([`BUILD.md`](BUILD.md)), your **MW2 PC multiplayer**
files, and for the Minecraft map an internet connection and `curl` the first
time. Jak's own model is optional: see [`JAK.md`](JAK.md) for the four GLBs;
without them the soldier and a slab stand in.

```bash
git fetch origin claude/tender-einstein-92k9z2 && git checkout claude/tender-einstein-92k9z2
cargo test -p approved_tests jak_parity     # no game needed: 20 passed
cp .env.example .env                        # set IW4L_GAMES to the folder holding your MW2 folder
set -a; . ./.env; set +a
make map mp_rust CMDS='spawn assault; force_match_start; jak on'
make map ZONE=minecraft:overworld CMDS='spawn assault; force_match_start; jak on'
```

Without `CMDS`, spawn and press **K** (or `` ` `` then `jak on`). Type
`jak debug` in the console for the state machine's readout: state, the
state before and what changed it, animation and frame, velocity, height,
the last jump's push and the trick latches. On an MW2
map the first start says "building Jak's collision for this map" for a few
seconds. `iw4l-artifacts/logs/latest.log` then shows `Jak collision: N
triangles`, `Jak Mode started`, `Jak: on the board`, and with the GLBs at map
load `Jak model: 289 clips, board true, gun true, 207 animations numbered
from .go art groups` (0 until the five `.go` files from [`JAK.md`](JAK.md) are
in the folder).

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
| exit | K or `jak off` | back to the MW2 soldier where Jak stood |

Not expected yet: Jak's model, sounds, zap or spin damage, grinding, hits on
MW2 players. K says "leave Skate first" while skating; E and Q do nothing in
the Minecraft world while Jak Mode is on.
