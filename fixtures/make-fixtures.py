#!/usr/bin/env python3
"""Write the conformance canvases into a vault.

They have to live in a vault for Obsidian to open them, which is why they are
generated rather than committed as-is:

    python3 fixtures/make-fixtures.py ~/vault/_canvas-conformance
"""

import json
import pathlib
import sys

OUT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "_canvas-conformance")
OUT.mkdir(parents=True, exist_ok=True)


def canvas(name, nodes, edges=None):
    (OUT / f"{name}.canvas").write_text(
        json.dumps({"nodes": nodes, "edges": edges or []}, indent=1)
    )


def text(node_id, body, **rest):
    node = {"id": node_id, "type": "text", "text": body,
            "x": 0, "y": 0, "width": 420, "height": 300}
    node.update(rest)
    return node


canvas("markdown", [
    text(name, body, x=(i % 4) * 460, y=(i // 4) * 340)
    for i, (name, body) in enumerate([
        ("headings", "# H1 heading\n## H2 heading\n### H3 heading\n#### H4\n##### H5\n###### H6"),
        ("inline", "Plain, **bold**, *italic*, ***both***, ~~strike~~, ==highlight==, `code`."),
        ("links", "A [markdown link](https://example.com), a [[Wikilink]], and a #tag here."),
        ("lists", "- first\n- second\n  - nested\n- third"),
        ("ordered", "1. one\n2. two\n3. three"),
        ("tasks", "- [ ] unchecked\n- [x] checked"),
        ("quote", "> quoted line\n> second line"),
        ("callout", "> [!note] Note title\n> body of the callout"),
        ("fence", "```rust\nfn main() {\n    println!(\"hi\");\n}\n```"),
        ("table", "| a | b |\n| --- | --- |\n| 1 | 2 |"),
        ("rule", "above\n\n---\n\nbelow"),
        ("math", "inline $x^2$ and block:\n\n$$a + b$$"),
        ("wrap", "A long paragraph that must wrap across several lines so line "
                 "breaking can be compared precisely against the application."),
        ("mixed", "## Title\nfirst line\nsecond line\n\nnew paragraph"),
    ])
])

canvas("unicode", [
    text("cjk", "## 日本語の見出し\n日本語のテキストが折り返されるかどうかを確認します。"),
    text("emoji", "## Emoji ✅ 🎉\nmixed ✅ text 🚀 with emoji 🧪 inline", x=470),
    text("rtl", "## עברית\nשלום עולם, זהו טקסט מימין לשמאל.", x=940),
    text("combining", "## Combining\nZalgo á̀̂ and é vs é", x=1410),
])

canvas("extremes", [
    text("tiny", "tiny", x=0, y=0, width=60, height=40),
    text("empty", "", x=120, y=0, width=200, height=120),
    text("blank", "   \n  \n", x=360, y=0, width=200, height=120),
    text("longword", "Supercalifragilisticexpialidociousandthensomemoretomakeitlonger",
         x=600, y=0, width=200, height=140),
    text("url", "https://example.com/a/very/long/path/that/cannot/break/anywhere/at/all",
         x=840, y=0, width=220, height=140),
    text("overflow", "## Overflow\n" + "\n".join(f"line {i}" for i in range(20)),
         x=1100, y=0, width=260, height=140),
    text("negative", "negative coords", x=-400, y=-260, width=240, height=120),
    text("wide", "a very wide node " * 6, x=0, y=220, width=1200, height=100),
])

canvas("nesting", [
    {"id": "outer", "type": "group", "label": "outer",
     "x": -40, "y": -40, "width": 900, "height": 560},
    {"id": "inner", "type": "group", "label": "inner", "color": "4",
     "x": 0, "y": 20, "width": 500, "height": 420},
    text("deeplist", "- one\n  - two\n    - three\n      - four\n- back to one",
         x=40, y=60, width=420, height=340),
    text("mixedlist", "1. first\n2. second\n   - bullet under ordered\n3. third",
         x=520, y=60, width=300, height=340),
])

canvas("inline2", [
    text("escapes", r"Escaped \*not italic\* and \*\*not bold\*\* and \`not code\`"),
    text("adjacent", "**bold**_italic_`code`~~strike~~ with no spaces between", x=470),
    text("nested", "**bold with *italic* inside** and *italic with **bold** inside*", x=940),
    text("awkward", "snake_case_word stays, a*b*c emphasises, 5 * 3 = 15 does not", x=1410),
    text("codespan", "``code with ` backtick`` and `a * b` inside code", x=0, y=340),
    text("linkstyle", "[**bold link**](https://x.com) and *[italic link](https://y.com)*",
         x=470, y=340),
])

# Whitespace beside an inline style. The plain text after `**bold**` is a run
# that begins with a space, and an SVG viewer collapses whitespace at the
# start of a text element, so the run's glyphs slide left by one space unless
# the renderer draws from the first visible glyph. The run's rect starts at
# the space either way, which is why extract-fixture.mjs records where the
# first visible glyph sits (`vx`) as well as where the run starts (`x`).
canvas("whitespace", [
    text("after-bold", "**bold** then plain"),
    text("after-italic", "*italic* then plain", x=470),
    text("after-code", "`code` then plain", x=940),
    text("after-strike", "~~strike~~ then plain", x=1410),
    text("after-mark", "==mark== then plain", x=0, y=340),
    text("after-link", "[[Wikilink]] then plain", x=470, y=340),
    text("between", "**one** **two** and *three* *four* end", x=940, y=340),
    text("paren", "**Control plane** (PocketBase) and `POST /token` issues", x=1410, y=340),
])

# array order is paint order, so the group must cover the node written before it
canvas("zorder", [
    text("under", "## under\nshould be behind the group", x=0, y=0, width=400, height=200),
    {"id": "over-group", "type": "group", "label": "group painted after",
     "x": 100, "y": 60, "width": 400, "height": 200},
    text("above", "## above\npainted last", x=250, y=120, width=400, height=200),
])

edge_nodes, edge_list = [], []
for i, from_side in enumerate(["top", "right", "bottom", "left"]):
    for j, to_side in enumerate(["top", "right", "bottom", "left"]):
        a, b = f"a{i}{j}", f"b{i}{j}"
        ox, oy = j * 700, i * 400
        edge_nodes += [text(a, from_side, x=ox, y=oy, width=200, height=120),
                       text(b, to_side, x=ox + 380, y=oy + 160, width=200, height=120)]
        edge_list.append({"id": f"e{i}{j}", "fromNode": a, "fromSide": from_side,
                          "toNode": b, "toSide": to_side,
                          "label": f"{from_side[0]}->{to_side[0]}"})
edge_nodes += [text("z1", "both ends", x=0, y=1700, width=200, height=120),
               text("z2", "arrows", x=400, y=1700, width=200, height=120),
               text("z3", "no arrow", x=800, y=1700, width=200, height=120)]
edge_list += [{"id": "ez", "fromNode": "z1", "toNode": "z2",
               "fromEnd": "arrow", "toEnd": "arrow", "color": "5"},
              {"id": "ez2", "fromNode": "z2", "toNode": "z3", "toEnd": "none"}]
canvas("edges", edge_nodes, edge_list)

# File and link nodes need something real to point at, so the note and the
# image are written here too rather than borrowed from whatever vault this
# lands in.
(OUT / "embedded-note.md").write_text(
    "# Embedded note\n\n"
    "First paragraph of the embedded note, long enough to wrap inside the node\n"
    "it is displayed in.\n\n"
    "- a list item\n- another\n"
)


def placeholder_png(path, width=320, height=200):
    """A test image, written without a codec so this script needs no library.

    Deliberately not a flat colour: the quadrants differ and a border runs
    round the edge, so a render shows whether the image was fitted, stretched,
    or flipped. Its 8:5 aspect ratio does not match the node it sits in, which
    is what makes the fitting visible at all.
    """
    import struct
    import zlib

    quadrants = ((198, 78, 76), (216, 151, 63), (68, 207, 110), (83, 132, 223))
    rows = []
    for y in range(height):
        row = bytearray(b"\x00")
        for x in range(width):
            edge = x < 4 or y < 4 or x >= width - 4 or y >= height - 4
            if edge:
                row += bytes((240, 240, 240))
            else:
                row += bytes(quadrants[(y >= height // 2) * 2 + (x >= width // 2)])
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(tag, payload):
        return (struct.pack(">I", len(payload)) + tag + payload
                + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF))

    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


placeholder_png(OUT / "sample-image.png")

folder = OUT.name
canvas("nodes", [
    {"id": "filenote", "type": "file", "file": f"{folder}/embedded-note.md",
     "x": 0, "y": 0, "width": 420, "height": 320},
    {"id": "fileimg", "type": "file", "file": f"{folder}/sample-image.png",
     "x": 470, "y": 0, "width": 420, "height": 320},
    {"id": "linknode", "type": "link", "url": "https://jsoncanvas.org",
     "x": 940, "y": 0, "width": 420, "height": 320},
    {"id": "filesub", "type": "file", "file": f"{folder}/embedded-note.md",
     "subpath": "#Embedded note", "x": 0, "y": 380, "width": 420, "height": 260},
])

print(f"wrote {len(list(OUT.glob('*.canvas')))} canvases to {OUT}")
