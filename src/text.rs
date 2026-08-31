//! Text measurement and rasterisation.
//!
//! Wrapping has to happen here rather than in the SVG, because SVG text does
//! not wrap. Measuring with the same font the app uses is what keeps line
//! breaks in the same places.

use cosmic_text::{fontdb, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, Weight, Wrap};

/// Inter, as Obsidian bundles it, instantiated at the weights the theme uses.
/// Embedding it is what makes the binary self-contained: no system font is
/// consulted, so measurement here matches measurement in the app.
const INTER: [&[u8]; 6] = [
    include_bytes!("../assets/Inter-400.ttf"),
    include_bytes!("../assets/Inter-600.ttf"),
    include_bytes!("../assets/Inter-700.ttf"),
    include_bytes!("../assets/Inter-400i.ttf"),
    include_bytes!("../assets/Inter-600i.ttf"),
    include_bytes!("../assets/Inter-700i.ttf"),
];

pub struct TextEngine {
    fonts: FontSystem,
    family: String,
}

impl TextEngine {
    pub fn new(family: &str) -> Self {
        let mut db = fontdb::Database::new();
        for face in INTER {
            db.load_font_data(face.to_vec());
        }
        // Inter carries no CJK, Hebrew, or emoji, and a missing glyph measures
        // as zero width — which silently defeats wrapping as well as drawing.
        // System faces are loaded behind the embedded ones: Latin still comes
        // from Inter and stays exact, everything else at least has glyphs.
        db.load_system_fonts();
        let fonts = FontSystem::new_with_locale_and_db("en-US".to_string(), db);
        Self { fonts, family: family.to_string() }
    }

    /// Height of the text box itself (ascent + descent) at a size — what a
    /// browser reports as the height of a text rect.
    pub fn text_height(&mut self, size: f32) -> f32 {
        let mut buffer = Buffer::new(&mut self.fonts, Metrics::new(size, size));
        let attrs = Attrs::new().family(Family::Name(&self.family));
        buffer.set_text(&mut self.fonts, "Ag", attrs, Shaping::Advanced);
        buffer.layout_runs().next().map(|r| r.line_height).unwrap_or(size * 1.19)
    }

    /// Where the baseline sits inside a line box, for a size and line height.
    /// cosmic-text places the baseline the same way CSS does — half the
    /// leading above the text — so this is its answer rather than a constant
    /// of mine.
    pub fn baseline_offset(&mut self, size: f32, line_height: f32) -> f32 {
        let mut buffer = Buffer::new(&mut self.fonts, Metrics::new(size, line_height));
        let attrs = Attrs::new().family(Family::Name(&self.family));
        buffer.set_text(&mut self.fonts, "Ag", attrs, Shaping::Advanced);
        buffer
            .layout_runs()
            .next()
            .map(|r| r.line_y - r.line_top)
            .unwrap_or(size * 0.9688)
    }

    /// Width of one unwrapped line.
    pub fn measure_styled(&mut self, text: &str, size: f32, weight: u16, italic: bool) -> f32 {
        let metrics = Metrics::new(size, size * 1.3);
        let mut buffer = Buffer::new(&mut self.fonts, metrics);
        let attrs = Attrs::new()
            .family(Family::Name(&self.family))
            .weight(Weight(weight))
            .style(if italic { Style::Italic } else { Style::Normal });
        buffer.set_wrap(&mut self.fonts, Wrap::None);
        buffer.set_size(&mut self.fonts, None, None);
        buffer.set_text(&mut self.fonts, text, attrs, Shaping::Advanced);
        buffer.layout_runs().map(|r| r.line_w).fold(0.0_f32, f32::max)
    }

    pub fn measure(&mut self, text: &str, size: f32, weight: u16) -> f32 {
        let metrics = Metrics::new(size, size * 1.3);
        let mut buffer = Buffer::new(&mut self.fonts, metrics);
        let attrs = Attrs::new()
            .family(Family::Name(&self.family))
            .weight(Weight(weight));
        buffer.set_wrap(&mut self.fonts, Wrap::None);
        buffer.set_size(&mut self.fonts, None, None);
        buffer.set_text(&mut self.fonts, text, attrs, Shaping::Advanced);
        buffer
            .layout_runs()
            .map(|run| run.line_w)
            .fold(0.0_f32, f32::max)
    }

    /// Break `text` to `width`, returning the lines in order.
    pub fn wrap(&mut self, text: &str, size: f32, weight: u16, width: f32) -> Vec<String> {
        if text.is_empty() {
            return Vec::new();
        }
        let metrics = Metrics::new(size, size * 1.3);
        let mut buffer = Buffer::new(&mut self.fonts, metrics);
        let attrs = Attrs::new()
            .family(Family::Name(&self.family))
            .weight(Weight(weight));
        buffer.set_wrap(&mut self.fonts, Wrap::WordOrGlyph);
        buffer.set_size(&mut self.fonts, Some(width.max(1.0)), None);
        buffer.set_text(&mut self.fonts, text, attrs, Shaping::Advanced);

        buffer
            .layout_runs()
            .map(|run| {
                let (start, end) = run
                    .glyphs
                    .iter()
                    .fold((usize::MAX, 0), |(s, e), g| (s.min(g.start), e.max(g.end)));
                if start == usize::MAX {
                    String::new()
                } else {
                    text.get(start..end).unwrap_or_default().trim().to_string()
                }
            })
            .filter(|line| !line.is_empty())
            .collect()
    }
}

/// Inter, inlined so the SVG carries its own typeface. Without this the file
/// only renders correctly on a machine that happens to have Inter installed,
/// which is not what "self-contained" should mean.
pub fn font_face_css() -> String {
    use std::fmt::Write as _;
    let mut style = String::from("<defs><style type=\"text/css\">");
    for (weight, italic, data) in [
        (400, false, INTER[0]), (600, false, INTER[1]), (700, false, INTER[2]),
        (400, true, INTER[3]), (600, true, INTER[4]), (700, true, INTER[5]),
    ] {
        let _ = write!(
            style,
            "@font-face{{font-family:'Inter';font-style:{slant};font-weight:{weight};\
src:url('data:font/ttf;base64,{data}') format('truetype');}}",
            slant = if italic { "italic" } else { "normal" },
            weight = weight,
            data = base64(data)
        );
    }
    style.push_str("</style></defs>");
    style
}

/// Minimal base64; pulling a crate in for one call is not worth the dependency.
pub fn base64_of(bytes: &[u8]) -> String { base64(bytes) }

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - i * 6)) & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Rasterise the SVG this program produced.
pub fn write_png(svg: &str, path: &str, scale: f32) {
    let mut options = resvg::usvg::Options::default();
    {
        let db = options.fontdb_mut();
        for face in INTER {
            db.load_font_data(face.to_vec());
        }
        // Same reason as the measuring engine: without these, any script
        // Inter does not cover rasterises as tofu even though it measured.
        db.load_system_fonts();
    }

    let tree = match resvg::usvg::Tree::from_str(svg, &options) {
        Ok(tree) => tree,
        Err(err) => {
            eprintln!("cannot rasterise: {err}");
            std::process::exit(1);
        }
    };
    let size = tree.size();
    let (w, h) = ((size.width() * scale) as u32, (size.height() * scale) as u32);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).expect("pixmap");
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png(path).expect("write png");
}
