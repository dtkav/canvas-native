# canvas-skill

A skill that lets an agent write and edit [JSON Canvas](https://jsoncanvas.org)
files, the format Obsidian Canvas uses, and then look at what it produced.

Every node in a canvas carries an absolute rect. Text does not reflow, boxes do
not grow to fit their contents, and the file records nothing when a label
overruns its box or an edge crosses a group. An agent writing one gets no
feedback at all, so it ships overlapping boxes and reports that the layout
looks fine.

This repository holds a skill file covering the format and the places it goes
wrong, and a renderer the agent runs after each edit.

## Install

```bash
cargo build --release
cp target/release/canvas-render ~/.local/bin/
```

Then register this directory with your agent as a skill, or copy `SKILL.md`
into wherever it reads skills from. Inter ships in `assets/`, so the binary
draws correct text on a machine with no fonts installed and no Obsidian.

## Use

```bash
canvas-render file.canvas out.svg --agent
canvas-render file.canvas out.png --agent
```

The SVG lists every box, curve, and text run with its coordinates, and stays
small enough for the agent to read as text. The PNG shows crowding and
collision, which coordinates alone will not settle. `--agent` leaves the fonts
out, since 2.8 MB of base64 buries the markup, and caps the image below the
size at which viewers begin rescaling.

After editing part of a large canvas, `--focus a,b,c` crops to those nodes and
scales the crop up, so the agent examines that corner at magnification instead
of hunting for it in a full render.

For a person to read:

```bash
canvas-render file.canvas diagram.png            # 2x pixel ratio
canvas-render file.canvas diagram.svg            # Inter embedded
canvas-render file.canvas diagram.svg --light    # light theme
```

## Fidelity

An approximate render misleads an agent worse than no render, because the agent
believes what it sees. An early lookalike drew group labels inside the box and
placed edge labels at the straight-line midpoint. It went on to report overlaps
in a file that had none.

The constants in `assets/theme-*.json` therefore come from measuring the
running application. `verify-native.mjs` diffs the edge paths this renderer emits
against the ones Obsidian draws, and finds them identical across all sixteen
side pairs. `NOTES.md` records what still differs; optical sizing at heading
sizes accounts for most of it.

## Limits

Text advances run about 2% wide at heading sizes, which can move a wrap point
on a long heading. Math renders as its source text. Code blocks arrive without
syntax highlighting, `link` nodes show the URL rather than the page title, and
group backgrounds are ignored.

## Licensing

MIT, in `LICENSE`.

The binary embeds six instances of Inter (400/600/700, upright and italic)
under the SIL Open Font License 1.1 by The Inter Project Authors.
`assets/Inter-LICENSE.txt` carries that licence, as the OFL requires.

Obsidian is a trademark of Dynalist Inc. This project has no affiliation with
them and contains none of their code.
