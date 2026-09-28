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

The shipping instance is pinned at `opsz=16`, so 16px body text is right and
headings (17 to 26px) run about 2% wide. Chrome opens the axis as the size
grows, which narrows the advances. A wrong advance moves the wrap point, and
every line after it inherits the shift.

Ways out, in order of preference:

1. A text stack that applies variation axes at shaping time. cosmic-text 0.12
   does not expose variation coordinates; candidates are parley, or swash
   directly, or rustybuzz plus our own line breaker.
2. One static instance per size the theme uses: 7 sizes x 6 weight/slant
   combinations is roughly 18 MB of embedded font.
3. Leave it, and accept the heading difference.

Verify with `compare.mjs`, which reports per-run x drift against the app.

## Fixtures

`fixtures/make-fixtures.py` writes nine conformance canvases into a vault,
along with the note and image the `file` nodes point at: markdown constructs,
node types, edges (all sixteen side pairs), unicode, extremes, nesting, inline
edge cases, whitespace beside inline styles, and z-order.
`extract-fixture.mjs` records what Obsidian rendered for each, including
where each run's first visible glyph sits; `compare.mjs` diffs this renderer
against that.

## Inline code and highlight spans

Obsidian gives a `code` or `mark` span horizontal padding and sets code in
its monospace face at 14px; this renderer draws both flush and measures code
with Inter. Every run after such a span on the same line starts a few pixels
early, and a line that is close to its width can wrap one word later than the
app. The `whitespace` fixture's `after-code`, `after-mark`, and `paren`
nodes show the drift.

## Not implemented

- math renders as its source text
- code syntax highlighting (Obsidian emits Prism tokens per language)
- group `background` / `backgroundStyle`
- `link` nodes show the URL; Obsidian shows the fetched page title
