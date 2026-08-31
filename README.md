# canvas-skill

An agent skill for writing and editing [JSON Canvas](https://jsoncanvas.org)
files — the format Obsidian Canvas uses.

Authoring a canvas is placing boxes by arithmetic. Every node carries an
absolute `x/y/width/height`, nothing reflows, and nothing reports that a label
overran its box or that an edge now cuts through a group. An agent writing one
is working blind, and blind authors produce diagrams with overlaps they then
describe as fine.

This closes the loop. It is a skill file that teaches the format and its
pitfalls, and a renderer the agent can run after every edit to see what it
just wrote.

## Install

```bash
cargo build --release
mkdir -p ~/.claude/skills/canvas
cp SKILL.md target/release/canvas-render ~/.claude/skills/canvas/
```

No system dependencies, and nothing to install alongside it: Inter is embedded,
so text is correct on a machine with no fonts and no Obsidian.

## What the agent gets

**The format, and where it bites.** Z-order is array order, so a group has to
precede its members. Text starts 17px down and 16px in, so the wrapping width
is `width - 32`. Content that overruns the height is clipped, not scrolled. A
group's label is drawn *above* its box. An edge leaves a face along its normal
and reaches out `clamp(distance / 2, 70, 150)` before curving, so the sides you
pick decide whether it runs clean or loops across a neighbour.

**Two ways to look at the result**, because they answer different questions:

```bash
canvas-render file.canvas /tmp/c.svg --agent   # positions, as readable text
canvas-render file.canvas /tmp/c.png --agent   # appearance, which needs an image
```

The SVG carries every box, curve, and text run with its coordinates, small
enough to read directly. The PNG is the only way to judge crowding and
collision. `--agent` drops the embedded fonts and sizes the image below the
point where viewers rescale it, since rescaling shifts every coordinate away
from the ones in the SVG.

**A way to check one part after an edit** — `--focus a,b,c` crops to those
nodes and scales up, so a corner of a large diagram is examined at
magnification rather than as a postage stamp.

## Producing one for a person

```bash
canvas-render file.canvas diagram.png            # 2x pixel ratio
canvas-render file.canvas diagram.svg            # Inter embedded, so it travels
canvas-render file.canvas diagram.svg --light    # light theme
```

## Fidelity

The render has to be the real thing. An approximation is worse than none,
because the agent believes it: an early lookalike put group labels inside the
box and edge labels at the straight-line midpoint, and produced confident,
wrong reports about a file that was fine.

So the constants are measured off the application rather than guessed, and the
match is checked rather than asserted — edge paths are identical across all
sixteen side pairs. `NOTES.md` records what still differs, chiefly optical
sizing at heading sizes.

## Licensing

MIT; see `LICENSE`.

**Inter** is embedded, six instances (400/600/700, upright and italic), under
the SIL Open Font License 1.1 by The Inter Project Authors. The licence is at
`assets/Inter-LICENSE.txt` and travels with the font, as the OFL requires.

**Obsidian** is a trademark of Dynalist Inc. This project is not affiliated
with or endorsed by them, and contains no Obsidian code.
