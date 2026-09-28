//! Render an Obsidian `.canvas` file to SVG or PNG, with no browser.
//!
//! Box positions come from the canvas file itself, so the only thing a CSS
//! engine would contribute is an enumerable set of values — colours, radii,
//! font sizes, spacing. Those were extracted once from a running Obsidian at
//! zoom multiplier 1 into `assets/theme-dark.json`, so this matches the app
//! without re-running its stylesheet.
//!
//! Edge geometry is Obsidian's own, verified path-for-path against the app:
//!   anchor   side midpoint pushed `endOffset` outward
//!   control  perpendicular, clamp(distance / 2, cpMin, cpMax)
//!   stub     drawn only on an end with no arrow

use serde::Deserialize;
use serde::Serialize as _;
use std::collections::HashMap;
use std::fmt::Write as _;

mod layout;
mod markdown;
mod text;

use markdown::parse_blocks;
use text::TextEngine;

#[derive(Deserialize)]
struct Canvas {
    #[serde(default)]
    nodes: Vec<Node>,
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize, Clone)]
struct Node {
    id: String,
    #[serde(default, rename = "type")]
    kind: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    subpath: Option<String>,
    #[serde(default)]
    color: Option<String>,
}

#[derive(Deserialize)]
struct Edge {
    #[serde(rename = "fromNode")]
    from_node: String,
    #[serde(rename = "toNode")]
    to_node: String,
    #[serde(default, rename = "fromSide")]
    from_side: Option<String>,
    #[serde(default, rename = "toSide")]
    to_side: Option<String>,
    #[serde(default, rename = "fromEnd")]
    from_end: Option<String>,
    #[serde(default, rename = "toEnd")]
    to_end: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    color: Option<String>,
}

#[derive(Deserialize)]
struct Theme {
    background: String,
    node: NodeStyle,
    text: String,
    font: String,
    headings: Vec<TypeStyle>,
    paragraph: TypeStyle,
    #[serde(rename = "blockMargin")]
    block_margin: f64,
    #[serde(rename = "groupLabel")]
    group_label: GroupLabel,
    edge: EdgeStyle,
    #[serde(rename = "edgeLabel")]
    edge_label: EdgeLabel,
    mono: String,
    link: String,
    highlight: String,
    #[serde(rename = "tagText")]
    tag_text: String,
    #[serde(rename = "tagBg")]
    tag_bg: String,
    #[serde(rename = "nodeLabel")]
    node_label: NodeLabel,
    #[serde(rename = "inlineTitle")]
    inline_title: InlineTitle,
    colors: HashMap<String, String>,
}

#[derive(Deserialize)]
struct NodeStyle {
    bg: String,
    border: String,
    #[serde(rename = "borderWidth")]
    border_width: f64,
    radius: f64,
    #[serde(rename = "padX")]
    pad_x: f64,
}

#[derive(Deserialize)]
struct NodeLabel {
    size: f64,
    color: String,
    line: f64,
    gap: f64,
}

#[derive(Deserialize)]
struct InlineTitle {
    size: f64,
    weight: u16,
    line: f64,
    #[serde(rename = "marginBottom")]
    margin_bottom: f64,
}

#[derive(Deserialize, Clone, Copy)]
struct TypeStyle {
    size: f64,
    weight: u16,
    line: f64,
}

#[derive(Deserialize)]
struct GroupLabel {
    size: f64,
    color: String,
    radius: f64,
    #[serde(rename = "padX")]
    pad_x: f64,
    #[serde(rename = "padY")]
    pad_y: f64,
    gap: f64,
}

#[derive(Deserialize)]
struct EdgeStyle {
    stroke: String,
    width: f64,
    #[serde(rename = "endOffset")]
    end_offset: f64,
    #[serde(rename = "cpMin")]
    cp_min: f64,
    #[serde(rename = "cpMax")]
    cp_max: f64,
    arrow: Vec<[f64; 2]>,
}

#[derive(Deserialize)]
struct EdgeLabel {
    size: f64,
    color: String,
    bg: String,
    radius: f64,
    pad: f64,
    line: f64,
}

#[derive(Clone, Copy)]
struct Anchor {
    x: f64,
    y: f64,
    ux: f64,
    uy: f64,
}

fn side(node: &Node, name: &str) -> Anchor {
    match name {
        "top" => Anchor { x: node.x + node.width / 2.0, y: node.y, ux: 0.0, uy: -1.0 },
        "bottom" => Anchor { x: node.x + node.width / 2.0, y: node.y + node.height, ux: 0.0, uy: 1.0 },
        "left" => Anchor { x: node.x, y: node.y + node.height / 2.0, ux: -1.0, uy: 0.0 },
        _ => Anchor { x: node.x + node.width, y: node.y + node.height / 2.0, ux: 1.0, uy: 0.0 },
    }
}

/// Obsidian picks the pair of faces the two nodes most directly present.
fn infer_sides(a: &Node, b: &Node) -> (&'static str, &'static str) {
    let dx = b.x + b.width / 2.0 - (a.x + a.width / 2.0);
    let dy = b.y + b.height / 2.0 - (a.y + a.height / 2.0);
    if dx.abs() > dy.abs() {
        if dx > 0.0 { ("right", "left") } else { ("left", "right") }
    } else if dy > 0.0 {
        ("bottom", "top")
    } else {
        ("top", "bottom")
    }
}

struct EdgeGeometry {
    path: String,
    tail: Anchor,
    tip: Anchor,
    from_arrow: bool,
    to_arrow: bool,
    from_angle: f64,
    to_angle: f64,
    mid: (f64, f64),
    hull: (f64, f64, f64, f64),
}

fn edge_geometry(edge: &Edge, nodes: &HashMap<String, Node>, style: &EdgeStyle) -> Option<EdgeGeometry> {
    let from = nodes.get(&edge.from_node)?;
    let to = nodes.get(&edge.to_node)?;
    let (auto_from, auto_to) = infer_sides(from, to);
    let a = side(from, edge.from_side.as_deref().unwrap_or(auto_from));
    let b = side(to, edge.to_side.as_deref().unwrap_or(auto_to));

    let start = (a.x + a.ux * style.end_offset, a.y + a.uy * style.end_offset);
    let end = (b.x + b.ux * style.end_offset, b.y + b.uy * style.end_offset);
    let distance = ((end.0 - start.0).powi(2) + (end.1 - start.1).powi(2)).sqrt();
    let reach = (distance / 2.0).clamp(style.cp_min, style.cp_max);

    let cp1 = (start.0 + a.ux * reach, start.1 + a.uy * reach);
    let cp2 = (end.0 + b.ux * reach, end.1 + b.uy * reach);

    let from_arrow = edge.from_end.as_deref().unwrap_or("none") == "arrow";
    let to_arrow = edge.to_end.as_deref().unwrap_or("arrow") == "arrow";

    // Order and direction follow the application: the leading stub, then the
    // curve, then the trailing stub drawn outward to the node edge.
    let mut path = String::new();
    if !from_arrow {
        let _ = write!(path, "M{} {} L{} {} ", a.x, a.y, start.0, start.1);
    }
    let _ = write!(
        path, "M{},{} C{},{} {},{} {},{}",
        start.0, start.1, cp1.0, cp1.1, cp2.0, cp2.1, end.0, end.1
    );
    if !to_arrow {
        let _ = write!(path, " M{} {} L{} {}", end.0, end.1, b.x, b.y);
    }

    let mid = |p0: f64, p1: f64, p2: f64, p3: f64| 0.125 * p0 + 0.375 * p1 + 0.375 * p2 + 0.125 * p3;
    let xs = [start.0, cp1.0, cp2.0, end.0];
    let ys = [start.1, cp1.1, cp2.1, end.1];

    Some(EdgeGeometry {
        path,
        tail: a,
        tip: b,
        from_arrow,
        to_arrow,
        from_angle: (-a.uy).atan2(-a.ux).to_degrees() + 90.0,
        to_angle: (-b.uy).atan2(-b.ux).to_degrees() + 90.0,
        mid: (mid(start.0, cp1.0, cp2.0, end.0), mid(start.1, cp1.1, cp2.1, end.1)),
        hull: (
            xs.iter().cloned().fold(f64::MAX, f64::min),
            ys.iter().cloned().fold(f64::MAX, f64::min),
            xs.iter().cloned().fold(f64::MIN, f64::max),
            ys.iter().cloned().fold(f64::MIN, f64::max),
        ),
    })
}

/// The vault a canvas belongs to: the nearest ancestor holding `.obsidian`.
/// A `file` node's path is vault-relative, so without this there is nothing
/// to resolve it against.
fn vault_root(canvas: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut dir = canvas.parent()?;
    loop {
        if dir.join(".obsidian").is_dir() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

enum Content {
    Markdown { source: String, title: Option<String> },
    Image(std::path::PathBuf),
    Link(String),
    Empty,
}

fn node_content(node: &Node, vault: Option<&std::path::Path>) -> Content {
    match node.kind.as_str() {
        "text" | "" => Content::Markdown {
            source: node.text.clone().unwrap_or_default(),
            title: None,
        },
        "link" => Content::Link(node.url.clone().unwrap_or_default()),
        "file" => {
            let Some(rel) = node.file.as_ref() else { return Content::Empty };
            let path = vault.map(|v| v.join(rel)).unwrap_or_else(|| rel.into());
            let extension = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp") {
                return Content::Image(path);
            }
            let source = std::fs::read_to_string(&path).unwrap_or_default();
            // A subpath embeds one section, and shows no filename title.
            let (source, title) = match node.subpath.as_deref() {
                Some(sub) => (section_of(&source, sub.trim_start_matches('#')), None),
                None => (
                    source,
                    path.file_stem().map(|s| s.to_string_lossy().into_owned()),
                ),
            };
            Content::Markdown { source, title }
        }
        _ => Content::Empty,
    }
}

/// The slice of a note under one heading, up to the next heading of the same
/// or higher level.
fn section_of(source: &str, heading: &str) -> String {
    let mut out = Vec::new();
    let mut level = 0usize;
    for line in source.lines() {
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if level == 0 {
            if hashes > 0 && line[hashes..].trim() == heading.trim() {
                level = hashes;
                out.push(line);
            }
            continue;
        }
        if hashes > 0 && hashes <= level {
            break;
        }
        out.push(line);
    }
    out.join("\n")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn resolve<'a>(color: Option<&'a str>, theme: &'a Theme, fallback: &'a str) -> String {
    match color {
        None => fallback.to_string(),
        Some(c) if c.starts_with('#') => c.to_string(),
        Some(c) => theme.colors.get(c).cloned().unwrap_or_else(|| fallback.to_string()),
    }
}

/// One drawn run, in node-local coordinates, for comparison with the app.
#[derive(serde::Serialize)]
pub struct LaidRun {
    node: String,
    text: String,
    x: f64,
    /// Where the first visible glyph lands. Equal to `x` unless the run
    /// begins with whitespace.
    vx: f64,
    y: f64,
    size: f64,
    weight: u16,
    italic: bool,
}

/// How a render is being used. An agent reads the SVG source as text and
/// looks at the PNG, so fonts are left out (they would bury the markup in
/// megabytes of base64) and the image is sized to what it will actually be
/// shown at. A render for a person is the opposite on both counts.
pub struct Options {
    pub embed_fonts: bool,
    pub crop: Option<(f64, f64, f64, f64)>,
    pub max_edge: Option<f64>,
    pub scale: f64,
}

fn build_svg(
    canvas: &Canvas,
    theme: &Theme,
    engine: &mut TextEngine,
    layout_out: &mut Vec<LaidRun>,
    vault: Option<&std::path::Path>,
    options: &Options,
) -> (String, f64, f64) {
    let nodes: HashMap<String, Node> =
        canvas.nodes.iter().map(|n| (n.id.clone(), n.clone())).collect();

    let (mut min_x, mut min_y) = (f64::MAX, f64::MAX);
    let (mut max_x, mut max_y) = (f64::MIN, f64::MIN);
    for n in &canvas.nodes {
        min_x = min_x.min(n.x);
        // group labels sit above their box
        min_y = min_y.min(n.y - theme.group_label.size - theme.group_label.gap * 2.0);
        max_x = max_x.max(n.x + n.width);
        max_y = max_y.max(n.y + n.height);
    }
    let geometries: Vec<_> = canvas
        .edges
        .iter()
        .map(|e| (e, edge_geometry(e, &nodes, &theme.edge)))
        .collect();
    for (edge, g) in geometries.iter().flat_map(|(e, g)| g.as_ref().map(|g| (e, g))) {
        min_x = min_x.min(g.hull.0);
        min_y = min_y.min(g.hull.1);
        max_x = max_x.max(g.hull.2);
        max_y = max_y.max(g.hull.3);
        // a label is centred on the midpoint and can reach past every curve
        if let Some(label) = edge.label.as_ref().filter(|l| !l.is_empty()) {
            let half_w = (engine.measure(label, theme.edge_label.size as f32, 400) as f64
                + theme.edge_label.pad * 2.0) / 2.0;
            let half_h = (theme.edge_label.line + theme.edge_label.pad * 2.0) / 2.0;
            min_x = min_x.min(g.mid.0 - half_w);
            max_x = max_x.max(g.mid.0 + half_w);
            min_y = min_y.min(g.mid.1 - half_h);
            max_y = max_y.max(g.mid.1 + half_h);
        }
    }

    const PAD: f64 = 48.0;
    let (ox, oy, w, h) = match options.crop {
        Some((x, y, cw, ch)) => (x - PAD, y - PAD, cw + PAD * 2.0, ch + PAD * 2.0),
        None => (
            min_x - PAD,
            min_y - PAD,
            max_x - min_x + PAD * 2.0,
            max_y - min_y + PAD * 2.0,
        ),
    };

    let mut svg = String::new();
    let _ = write!(
        svg,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
            "{fonts}",
            r#"<rect width="100%" height="100%" fill="{bg}"/><g transform="translate({tx},{ty})">"#
        ),
        w = w, h = h,
        fonts = if options.embed_fonts { text::font_face_css() } else { String::new() },
        bg = theme.background, tx = -ox, ty = -oy
    );

    // JSON Canvas 1.0: "Nodes are placed in the array in ascending order by
    // z-index" — array order is the paint order, not node type.
    for node in &canvas.nodes {
        let stroke = resolve(node.color.as_deref(), theme, &theme.node.border);
        let is_group = node.kind == "group";
        let fill = if is_group { "none" } else { theme.node.bg.as_str() };
        let _ = write!(
            svg,
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}" stroke="{s}" stroke-width="{sw}"/>"#,
            x = node.x, y = node.y, w = node.width, h = node.height,
            r = theme.node.radius, fill = fill, s = stroke, sw = theme.node.border_width
        );
        if node.color.is_some() {
            let _ = write!(
                svg,
                r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{s}" fill-opacity="0.07"/>"#,
                x = node.x, y = node.y, w = node.width, h = node.height,
                r = theme.node.radius, s = stroke
            );
        }

        if is_group {
            if let Some(label) = node.label.as_ref().filter(|l| !l.is_empty()) {
                let gl = &theme.group_label;
                let width = engine.measure(label, gl.size as f32, 400) as f64 + gl.pad_x * 2.0;
                let height = gl.size + gl.pad_y * 2.0;
                let top = node.y - height - gl.gap;
                layout_out.push(LaidRun {
                    node: node.id.clone(),
                    text: label.clone(),
                    x: gl.pad_x,
                    vx: gl.pad_x,
                    y: top - node.y + gl.pad_y,
                    size: gl.size,
                    weight: 400,
                    italic: false,
                });
                let _ = write!(
                    svg,
                    concat!(
                        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{c}" fill-opacity="0.1"/>"#,
                        r#"<text x="{tx}" y="{ty}" font-family="{f}" font-size="{s}" fill="{c}">{t}</text>"#
                    ),
                    x = node.x, y = top, w = width, h = height, r = gl.radius, c = gl.color,
                    tx = node.x + gl.pad_x, ty = top + gl.pad_y + gl.size * 0.78,
                    f = theme.font, s = gl.size, t = escape(label)
                );
            }
            continue;
        }

        let content = node_content(node, vault);

        // Image and link nodes carry their name above the box, not inside it.
        if let Content::Image(_) | Content::Link(_) = content {
            let caption = match &content {
                Content::Image(path) => path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                Content::Link(url) => url.clone(),
                _ => String::new(),
            };
            let nl = &theme.node_label;
            let _ = write!(
                svg,
                r#"<text x="{x}" y="{y}" font-family="{f}" font-size="{s}" fill="{c}">{t}</text>"#,
                x = node.x, y = node.y - nl.gap, f = theme.font,
                s = nl.size, c = nl.color, t = escape(&caption)
            );
            layout_out.push(LaidRun {
                node: node.id.clone(),
                text: caption.clone(),
                x: 0.0,
                vx: 0.0,
                y: -nl.gap - nl.size * 0.82,
                size: nl.size,
                weight: 400,
                italic: false,
            });
        }

        let mut inline_title = None;
        let source = match &content {
            Content::Markdown { source, title } => {
                inline_title = title.clone();
                source.clone()
            }
            Content::Image(path) => {
                if let Ok(bytes) = std::fs::read(path) {
                    let mime = match path.extension().and_then(|e| e.to_str()) {
                        Some("png") => "image/png",
                        Some("gif") => "image/gif",
                        Some("webp") => "image/webp",
                        Some("svg") => "image/svg+xml",
                        _ => "image/jpeg",
                    };
                    let _ = write!(
                        svg,
                        r#"<image x="{x}" y="{y}" width="{w}" height="{h}" preserveAspectRatio="xMidYMid meet" href="data:{m};base64,{d}"/>"#,
                        x = node.x, y = node.y, w = node.width, h = node.height,
                        m = mime, d = text::base64_of(&bytes)
                    );
                }
                String::new()
            }
            Content::Link(url) => format!("{url}"),
            Content::Empty => String::new(),
        };
        let inner = node.width - theme.node.pad_x * 2.0;
        let metrics = layout::Metrics {
            heading: theme.headings.iter().map(|h| (h.size, h.weight, h.line)).collect(),
            paragraph: (theme.paragraph.size, theme.paragraph.weight, theme.paragraph.line),
            inline_title: (
                theme.inline_title.size,
                theme.inline_title.weight,
                theme.inline_title.line,
                theme.inline_title.margin_bottom,
            ),
        };
        let mut blocks = parse_blocks(&source);
        if let Some(name) = inline_title {
            blocks.insert(0, markdown::Block {
                kind: markdown::Kind::InlineTitle,
                runs: markdown::inline(&name),
            });
        }
        // Obsidian keeps overflowing content and clips it; a clip path says
        // the same thing without discarding the runs.
        let clip = format!("clip{}", node.id.replace(|c: char| !c.is_alphanumeric(), ""));
        let _ = write!(
            svg,
            r#"<clipPath id="{id}"><rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}"/></clipPath><g clip-path="url(#{id})">"#,
            id = clip, x = node.x, y = node.y, w = node.width, h = node.height, r = theme.node.radius
        );
        for line in layout::lay_out(engine, &blocks, &metrics, 0.0, inner) {
            for run in &line.runs {
                // Baseline within the line box, as cosmic-text computes it.
                let offset = engine.baseline_offset(run.size as f32, line.height as f32) as f64;
                let baseline = node.y + line.top + offset;
                let text_top = line.top
                    + (line.height - engine.text_height(run.size as f32) as f64) / 2.0;
                let mut local_x = theme.node.pad_x + run.x;
                let rtl = layout::is_rtl(&run.text);
                if rtl {
                    let w = engine.measure_styled(&run.text, run.size as f32, run.weight, run.italic) as f64;
                    local_x = node.width - theme.node.pad_x - w;
                }
                // An SVG viewer collapses whitespace at the start of a text
                // element, which would slide the run's first glyph left by
                // the width of that space: `**bold** then` would draw as
                // "boldthen". The element therefore starts at the first
                // visible glyph, moved right by what the whitespace measured,
                // and a run that is only whitespace draws nothing at all.
                // Trailing whitespace costs nothing since the next run is
                // placed absolutely, so it is trimmed as well.
                let shown = run.text.trim_start();
                if shown.is_empty() {
                    continue;
                }
                let lead = if rtl || shown.len() == run.text.len() {
                    0.0
                } else {
                    (engine.measure_styled(&run.text, run.size as f32, run.weight, run.italic)
                        - engine.measure_styled(shown, run.size as f32, run.weight, run.italic)) as f64
                };
                let drawn = shown.trim_end();
                let visible_x = local_x + lead;
                if !run.style.marker {
                    layout_out.push(LaidRun {
                        node: node.id.clone(),
                        text: run.text.clone(),
                        x: local_x,
                        vx: visible_x,
                        y: text_top,
                        size: run.size,
                        weight: run.weight,
                        italic: run.italic,
                    });
                }
                let x = node.x + visible_x;
                let fill = if run.style.tag {
                    theme.tag_text.as_str()
                } else if run.style.link {
                    theme.link.as_str()
                } else {
                    theme.text.as_str()
                };
                if run.style.tag {
                    let w = engine.measure_styled(&run.text, run.size as f32, run.weight, false) as f64;
                    let _ = write!(
                        svg,
                        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="4" fill="{c}"/>"#,
                        x = node.x + theme.node.pad_x + run.x - 4.0,
                        y = baseline - run.size * 0.82,
                        w = w + 8.0, h = run.size * 1.19, c = theme.tag_bg
                    );
                }
                let family = if run.style.code {
                    "Source Code Pro, monospace"
                } else {
                    "Inter, Noto Sans, Noto Sans CJK JP, Noto Color Emoji, sans-serif"
                };
                if run.style.mark {
                    let w = engine.measure_styled(drawn, run.size as f32, run.weight, run.italic) as f64;
                    let _ = write!(
                        svg,
                        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{c}"/>"#,
                        x = x, y = baseline - run.size * 0.82, w = w,
                        h = run.size * 1.19, c = theme.highlight
                    );
                }
                let _ = write!(
                    svg,
                    r#"<text x="{x}" y="{y}" font-family="{f}" font-size="{s}" font-weight="{wt}"{it} fill="{c}">{t}</text>"#,
                    x = x, y = baseline, f = family, s = run.size, wt = run.weight,
                    it = if run.italic { r#" font-style="italic""# } else { "" },
                    c = fill, t = escape(drawn)
                );
                if run.style.strike {
                    let w = engine.measure_styled(drawn, run.size as f32, run.weight, run.italic) as f64;
                    let _ = write!(
                        svg,
                        r#"<rect x="{x}" y="{y}" width="{w}" height="1.2" fill="{c}"/>"#,
                        x = x, y = baseline - run.size * 0.3, w = w, c = fill
                    );
                }
            }
        }
        svg.push_str("</g>");
    }

    let arrow_points: String = theme
        .edge
        .arrow
        .iter()
        .map(|p| format!("{},{}", p[0], p[1]))
        .collect::<Vec<_>>()
        .join(" ");

    for (edge, geometry) in &geometries {
        let Some(g) = geometry else { continue };
        let stroke = resolve(edge.color.as_deref(), theme, &theme.edge.stroke);
        let _ = write!(
            svg,
            r#"<path d="{d}" fill="none" stroke="{s}" stroke-width="{w}"/>"#,
            d = g.path, s = stroke, w = theme.edge.width
        );
        for (on, anchor, angle) in [(g.from_arrow, g.tail, g.from_angle), (g.to_arrow, g.tip, g.to_angle)] {
            if on {
                let _ = write!(
                    svg,
                    r#"<polygon points="{p}" fill="{s}" transform="translate({x},{y}) rotate({a})"/>"#,
                    p = arrow_points, s = stroke, x = anchor.x, y = anchor.y, a = angle
                );
            }
        }
    }

    for (edge, geometry) in &geometries {
        let (Some(g), Some(label)) = (geometry, edge.label.as_ref()) else { continue };
        if label.is_empty() {
            continue;
        }
        let el = &theme.edge_label;
        let bw = engine.measure(label, el.size as f32, 400) as f64 + el.pad * 2.0;
        let bh = el.line + el.pad * 2.0;
        // `.canvas-path-label` carries translate(-50%, -50%): centred on the midpoint.
        let (bx, by) = (g.mid.0 - bw / 2.0, g.mid.1 - bh / 2.0);
        let _ = write!(
            svg,
            concat!(
                r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{bg}"/>"#,
                r#"<text x="{tx}" y="{ty}" font-family="{f}" font-size="{s}" fill="{c}" text-anchor="middle">{t}</text>"#
            ),
            x = bx, y = by, w = bw, h = bh, r = el.radius, bg = el.bg,
            tx = g.mid.0, ty = by + el.pad + el.size * 0.78,
            f = theme.font, s = el.size, c = el.color, t = escape(label)
        );
    }

    svg.push_str("</g></svg>");
    (svg, w, h)
}

/// The bounding box of named nodes, for rendering just the part that changed.
fn focus_bounds(canvas: &Canvas, ids: &[String]) -> Option<(f64, f64, f64, f64)> {
    let chosen: Vec<&Node> = canvas
        .nodes
        .iter()
        .filter(|n| ids.iter().any(|id| id == &n.id))
        .collect();
    if chosen.is_empty() {
        return None;
    }
    let min_x = chosen.iter().map(|n| n.x).fold(f64::MAX, f64::min);
    let min_y = chosen.iter().map(|n| n.y).fold(f64::MAX, f64::min);
    let max_x = chosen.iter().map(|n| n.x + n.width).fold(f64::MIN, f64::max);
    let max_y = chosen.iter().map(|n| n.y + n.height).fold(f64::MIN, f64::max);
    Some((min_x, min_y, max_x - min_x, max_y - min_y))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let flag = |name: &str| args.iter().any(|a| a == name);
    let value = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    if positional.is_empty() || flag("--help") {
        eprintln!(
            "usage: canvas-native <file.canvas> [out.svg|out.png] [options]\n\n\
             \x20 --agent          for reading: SVG without embedded fonts, image\n\
             \x20                  sized so a viewer will not rescale it\n\
             \x20 --light          light theme\n\
             \x20 --focus a,b,c    render only these nodes and their surroundings\n\
             \x20 --region x,y,w,h render only this area, in canvas coordinates\n\
             \x20 --max <px>       cap the longest edge of the image\n\
             \x20 --scale <n>      pixel ratio when the size is not capped"
        );
        std::process::exit(2);
    }

    let input = positional[0].clone();
    let output = positional
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "canvas.svg".into());

    let raw = std::fs::read_to_string(&input).unwrap_or_else(|e| {
        eprintln!("cannot read {input}: {e}");
        std::process::exit(1);
    });
    let canvas: Canvas = serde_json::from_str(&raw).unwrap_or_else(|e| {
        eprintln!("{input} is not a canvas: {e}");
        std::process::exit(1);
    });
    let theme: Theme = serde_json::from_str(if flag("--light") {
        include_str!("../assets/theme-light.json")
    } else {
        include_str!("../assets/theme-dark.json")
    })
    .unwrap();

    let agent = flag("--agent");
    let crop = value("--region")
        .and_then(|r| {
            let parts: Vec<f64> = r.split(',').filter_map(|p| p.trim().parse().ok()).collect();
            (parts.len() == 4).then(|| (parts[0], parts[1], parts[2], parts[3]))
        })
        .or_else(|| {
            value("--focus").and_then(|ids| {
                let ids: Vec<String> = ids.split(',').map(|s| s.trim().to_string()).collect();
                focus_bounds(&canvas, &ids)
            })
        });

    let options = Options {
        // An agent reads the markup; base64 font blobs would bury it.
        embed_fonts: !agent && !flag("--no-embed-font"),
        crop,
        // 2000px is where common viewers start rescaling, which wastes
        // detail and shifts every coordinate the reader might reason about.
        max_edge: value("--max")
            .and_then(|m| m.parse().ok())
            .or(if agent { Some(2000.0) } else { None }),
        scale: value("--scale")
            .and_then(|s| s.parse().ok())
            .unwrap_or(if agent { 1.0 } else { 2.0 }),
    };

    let vault = vault_root(std::path::Path::new(&input));
    let mut engine = TextEngine::new(&theme.font);
    let mut laid = Vec::new();
    let (svg, width, height) =
        build_svg(&canvas, &theme, &mut engine, &mut laid, vault.as_deref(), &options);

    if output.ends_with(".layout.json") {
        std::fs::write(&output, serde_json::to_string_pretty(&laid).unwrap()).unwrap();
        println!("{output}  ({} runs)", laid.len());
        return;
    }

    // Fill the cap rather than merely staying under it: a focused region is
    // small in canvas units, and the whole point of asking for one is to see
    // it larger. Bounded so a tiny crop does not become absurd.
    let scale = match options.max_edge {
        Some(cap) => (cap / width.max(height)).clamp(0.1, 4.0),
        None => options.scale,
    };

    if output.ends_with(".png") {
        text::write_png(&svg, &output, scale as f32);
        println!(
            "{output}  {}x{}  ({} nodes, {} edges)",
            (width * scale).round(),
            (height * scale).round(),
            canvas.nodes.len(),
            canvas.edges.len()
        );
    } else {
        std::fs::write(&output, &svg).unwrap();
        println!(
            "{output}  {}x{}  ({} nodes, {} edges{})",
            width.round(),
            height.round(),
            canvas.nodes.len(),
            canvas.edges.len(),
            if options.embed_fonts { ", fonts embedded" } else { "" }
        );
    }
}
