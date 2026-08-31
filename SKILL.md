---
name: canvas
description: Write, edit, and check JSON Canvas (.canvas) files, the format Obsidian Canvas uses, and render them to SVG or PNG with a self-contained binary that needs no Obsidian and no browser. Use whenever authoring or modifying a canvas, checking a layout, or attaching a diagram, and in place of guessing at how a canvas looks.
---

# Canvas

Authoring a canvas means placing boxes by arithmetic. Every node carries an
absolute rect, nothing reflows, and the file records nothing when a label
overruns its box or an edge cuts through a group. Write one without looking and
it will carry overlaps you never see.

Render after every edit, and read the render.

```bash
canvas-render file.canvas /tmp/c.svg --agent   # positions, as text you can read
canvas-render file.canvas /tmp/c.png --agent   # appearance, which needs an image
```

The binary sits beside this file if it is not already on your PATH.

## The format

JSON Canvas 1.0, `{"nodes": [...], "edges": [...]}`.

Every node has `id`, `type`, `x`, `y`, `width`, `height`, and optionally
`color`. `y` grows downward. Array order sets z-order: later nodes paint over
earlier ones, so list a group before the nodes it contains.

| `type` | carries |
| --- | --- |
| `text` | `text`, markdown |
| `file` | `file`, a vault-relative path, plus an optional `subpath` like `#Heading` |
| `link` | `url` |
| `group` | an optional `label` |

Edges take `id`, `fromNode`, `toNode`, and optionally `fromSide` / `toSide`
(`top`, `right`, `bottom`, `left`), `fromEnd` / `toEnd` (`none` or `arrow`,
defaulting to an arrow on the `to` end), `label`, and `color`.

`color` takes `"1"` through `"6"` for red, orange, yellow, green, cyan, and
purple, or a hex string. Omit it and the node keeps the theme border.

## Sizing a node

Use the numbers rather than an estimate.

- Text starts 17px below the node top and sits 16px in from each side, so it
  wraps at `width - 32`.
- Body text is 16px on a 24px line. Headings run 19 to 26px and carry a 40px
  top margin, paragraphs 16px. Adjacent margins collapse to the larger.
- A group's label draws above its box, so leave room above a group rather than
  inside it.
- Content taller than the node is clipped and not scrolled, so the height has
  to fit whatever you put in.

## Routing an edge

An edge leaves the midpoint of a side, 7px out, and heads straight out from
that face before it curves. The control point sits `clamp(distance / 2, 70,
150)` along the face normal.

The sides therefore decide the shape and not only the endpoints. Two faces
pointing at each other give a smooth S. A face pointing away from its target
loops out by about three quarters of that reach before turning back, which
can carry the line across a box that looked well clear of it. Left unset, each
end picks the face it presents most squarely to the other node. Set `fromSide`
and `toSide` when that choice routes the line through something.

## Reading your own work

Render both forms. They answer different questions.

The SVG gives you positions: every box, curve, and text run with its
coordinates, small enough to read as text. The PNG gives you appearance,
covering overlap, crowding, and a label colliding with an edge.

`--agent` drops the embedded fonts, since 2.8 MB of base64 buries the markup,
and caps the image at 2000px, where common viewers start rescaling. Rescaling
wastes detail and shifts every coordinate away from the ones in the SVG.

After changing part of a canvas, render that part:

```bash
canvas-render file.canvas /tmp/c.png --agent --focus envoy,vault,tap
```

The crop takes the bounding box of those nodes plus their surroundings and
scales it up to fill the frame, so you examine a small region at magnification
instead of rendering it as a postage stamp.

Look before judging a layout. If you are about to say that a label overlaps or
an edge is clipped, render and look first. An approximation of the drawing
misleads you worse than no drawing, because you will believe it: an earlier
lookalike put group labels inside the box and edge labels at the straight-line
midpoint, then reported overlaps in a file that had none.

## Producing one for a person

```bash
canvas-render file.canvas diagram.png            # 2x pixel ratio
canvas-render file.canvas diagram.svg            # Inter embedded, so it travels
canvas-render file.canvas diagram.svg --light    # light theme
```

Send the SVG when someone may edit or scale it, the PNG when they will only
look at it.

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
on a long heading, so leave slack rather than sizing a box to a heading that
barely fits. Math renders as its source text. Code blocks arrive without
syntax highlighting, `link` nodes show the URL rather than the page title, and
group backgrounds are ignored.
