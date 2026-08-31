# canvas-render

Renders a [JSON Canvas](https://jsoncanvas.org) file — the format Obsidian
Canvas writes — to SVG or PNG. One self-contained binary: no Obsidian, no
browser, and no font installed on the machine.

```bash
canvas-render architecture.canvas diagram.png
canvas-render architecture.canvas diagram.svg --light
```

## Options

| | |
| --- | --- |
| `--light` | light theme; dark is the default |
| `--focus a,b,c` | render only these nodes and their surroundings |
| `--region x,y,w,h` | render only this area, in canvas coordinates |
| `--max <px>` | cap the longest edge of the image |
| `--scale <n>` | pixel ratio when the size is not capped |
| `--agent` | SVG without embedded fonts, image sized so no viewer rescales it |

The output format follows the extension. SVG embeds Inter as `@font-face`
data URIs by default, so the text is right on a machine with no fonts
installed; PNG renders at 2x.

`--focus` and `--region` crop to a part of the canvas and scale it up to fill
the frame, which is how you examine one corner of a large diagram without
rendering the whole thing and squinting.

`--agent` is for a program reading the output rather than a person looking at
it. It drops the embedded fonts, since 2.8 MB of base64 buries the markup a
reader wants, and caps the image where common viewers begin rescaling.

## What it renders

All four node types — `text`, `file`, `link`, `group` — in array order, so
z-order is the spec's. `file` nodes embed the note or image they point at,
resolved against the vault the canvas sits in.

Markdown covers headings, paragraphs, lists to any depth, quotes, callouts,
tables, code blocks, rules, and inline bold, italic, code, strikethrough,
highlight, wikilinks, external links, and tags.

Edges carry labels, arrowheads on either end, the six preset colours and
arbitrary hex, and the side-to-side routing Obsidian uses, including the
side it picks when the file does not name one.

## Building

```bash
cargo build --release
```

No system dependencies. Inter ships in `assets/`, so the binary is the only
artefact you need.

## Limits

Text advances run about 2% wide at heading sizes, which can move a wrap point
on a long heading. Math renders as its source text, code blocks are not syntax
highlighted, `link` nodes show the URL rather than the page title, and group
backgrounds are ignored. `NOTES.md` has the detail.

## Licensing

MIT; see `LICENSE`.

**Inter** is embedded, six instances (400/600/700, upright and italic), under
the SIL Open Font License 1.1 by The Inter Project Authors. The licence is at
`assets/Inter-LICENSE.txt` and travels with the font, as the OFL requires.

**Obsidian** is a trademark of Dynalist Inc. This project is not affiliated
with or endorsed by them, and contains no Obsidian code.
