# Known gaps

## Text metrics: optical sizing (deferred, wanted)

Inter is a variable font with an `opsz` axis, and Chrome drives it from the
font size (`font-optical-sizing: auto`). Static instances can only be correct
at one size:

| instance | `markdown link` at 16px |
| --- | --- |
| opsz=14 (axis default) | 110.23 |
| opsz=16 *(shipping)* | 109.22 |
| opsz=20 | 107.17 |
| Chrome | 107.42 |

Currently pinned at `opsz=16`, so 16px body text is right and headings
(17–26px) run ~2% wide — Chrome opens the axis as the size grows, which
narrows the advances. A wrong advance moves the wrap point, and every line
after it inherits the shift.

Ways out, in order of preference:

1. A text stack that applies variation axes at shaping time. cosmic-text 0.12
   does not expose variation coordinates; candidates are parley, or swash
   directly, or rustybuzz plus our own line breaker.
2. One static instance per size the theme uses — 7 sizes x 6 weight/slant
   combinations is roughly 18 MB of embedded font.
3. Leave it, and accept the heading difference.

Verify with `compare.mjs`, which reports per-run x drift against the app.

## Fixtures

`fixtures/make-fixtures.py` writes the conformance canvases into a vault,
along with the note and image the `file` nodes point at: markdown
constructs, node types, colours, edges (all sixteen side pairs), unicode,
extremes, nesting, inline edge cases, and z-order.
`extract-fixture.mjs` records what Obsidian rendered; `compare.mjs` diffs this
renderer against it.

## Not implemented

- math renders as its source text
- code syntax highlighting (Obsidian emits Prism tokens per language)
- group `background` / `backgroundStyle`
- `link` nodes show the URL; Obsidian shows the fetched page title
