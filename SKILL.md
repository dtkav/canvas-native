---
name: canvas
description: Write, edit, and check JSON Canvas (.canvas) files — the format Obsidian Canvas uses — and render them to SVG or PNG with a self-contained binary, no Obsidian and no browser. Use whenever authoring or modifying a canvas, checking a layout, or attaching a diagram, and instead of guessing at how a canvas looks.
---

# Canvas

Authoring a canvas is placing boxes by arithmetic. Every node carries an
absolute rect, nothing reflows, and nothing tells you a label overran its box
or that an edge now cuts through a group. Write one without looking and it
will have overlaps you never see.

So: render after every edit, and read the render.

```bash
CR=~/.claude/skills/canvas/canvas-render
$CR file.canvas /tmp/c.svg --agent   # positions, as text you can read
$CR file.canvas /tmp/c.png --agent   # appearance, which only an image shows
```

## The format

JSON Canvas 1.0 — `{"nodes": [...], "edges": [...]}`.

Every node has `id`, `type`, `x`, `y`, `width`, `height`, and optionally
`color`. `y` grows downward. **Array order is z-order**: later nodes paint
over earlier ones, so a group must be listed before the nodes it contains.

| `type` | carries |
| --- | --- |
| `text` | `text` — markdown |
| `file` | `file` — a vault-relative path; optional `subpath`, e.g. `#Heading` |
| `link` | `url` |
| `group` | optional `label` |

Edges take `id`, `fromNode`, `toNode`, and optionally `fromSide` / `toSide`
(`top`\|`right`\|`bottom`\|`left`), `fromEnd` / `toEnd` (`none`\|`arrow` —
arrow on the `to` end unless you say otherwise), `label`, and `color`.

`color` is `"1"`–`"6"` — red, orange, yellow, green, cyan, purple — or a hex
string. Omit it and the node uses the theme's own border.

## Sizing a node

This is where a blind author goes wrong, so use the numbers.

- Text starts **17px** below the node top and is inset **16px** each side, so
  the wrapping width is `width - 32`.
- Body text is 16px on a 24px line. Headings run 19–26px and carry a **40px
  top margin**; paragraphs carry 16px. Adjacent margins collapse to the larger.
- A **group's label sits above its box**, not inside it. Leave room above a
  group, not within it.
- Content that overruns the height is **clipped, not scrolled**. Height is a
  promise you have to keep.

## Routing an edge

An edge leaves the midpoint of a side, 7px out, and heads straight out from
that face before curving: the control point sits `clamp(distance / 2, 70, 150)`
along the face normal.

So the sides decide the shape, not just the endpoints. Two faces pointing at
each other give a smooth S. A face pointing *away* from its target loops out
by roughly three-quarters of that reach before turning back, which can carry
the line across a box that looked well clear of it. Left unset, each end picks
the face it most directly presents to the other node — usually what you want;
set `fromSide` / `toSide` when it isn't.

## Reading your own work

Render **both** forms; they answer different questions.

The SVG is ground truth for *positions*: every box, curve, and text run with
its coordinates, small enough to read as text. The PNG is the only way to
judge *appearance* — overlap, crowding, a label colliding with an edge.

`--agent` drops the embedded fonts, because 2.8 MB of base64 buries the
markup, and caps the image at 2000px, where common viewers start rescaling.
Being rescaled wastes detail and shifts every coordinate away from the ones
in the SVG.

After changing part of a canvas, render just that part:

```bash
$CR file.canvas /tmp/c.png --agent --focus envoy,vault,tap
```

The crop is the bounding box of those nodes plus surroundings, scaled up to
fill the frame, so a small region is examined at magnification instead of
rendering as a postage stamp.

**Look before judging a layout.** If you are about to say "that label
overlaps" or "that edge is clipped", render and look. An approximation of the
drawing is worse than none, because you will believe it: an earlier lookalike
put group labels inside the box and edge labels at the straight-line midpoint,
and produced confident, wrong reports about a file that was fine.

## Producing one for a person

```bash
$CR file.canvas diagram.png            # 2x pixel ratio
$CR file.canvas diagram.svg            # Inter embedded, so it travels
$CR file.canvas diagram.svg --light    # light theme
```

Send the SVG when it may be edited or scaled, the PNG when it will only be
looked at.

## Options

| | |
| --- | --- |
| `--agent` | for reading: SVG without embedded fonts, image sized so no viewer rescales it |
| `--light` | light theme |
| `--focus a,b,c` | render only these nodes and their surroundings |
| `--region x,y,w,h` | render only this area, in canvas coordinates |
| `--max <px>` | cap the longest edge of the image |
| `--scale <n>` | pixel ratio when the size is not capped |

## Limits

Text advances run about 2% wide at heading sizes, which can move a wrap point
on a long heading — so leave a little slack rather than sizing a box to a
heading that only just fits. Math renders as its source text, code blocks are
not syntax highlighted, `link` nodes show the URL rather than the page title,
and group backgrounds are ignored.
