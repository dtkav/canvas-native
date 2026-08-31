//! Turn parsed blocks into positioned lines.
//!
//! The spacing model is read off the application (see `fixtures/`):
//!
//!   content top      17.09
//!   heading          margin-top 40, margin-bottom 0
//!   paragraph        margin-top 16, margin-bottom 0
//!   rule             margin 32 both sides
//!   list item        line box 26.37, no margin between items
//!
//! Adjacent margins collapse, so the gap is the larger of the two rather than
//! their sum. That reproduces every gap observed: heading to heading 40,
//! heading to paragraph 16, paragraph to paragraph 16, paragraph to rule 32,
//! and rule to paragraph 32 — which an additive model cannot do at once.

use crate::markdown::{Block, Kind, Run, Style};
use crate::text::TextEngine;

pub const CONTENT_TOP: f64 = 17.09;
pub const LIST_INDENT: f64 = 26.78;
pub const LIST_LINE: f64 = 26.37;
pub const QUOTE_INDENT: f64 = 18.0;
pub const CALLOUT_INDENT: f64 = 25.0;
pub const CALLOUT_TITLE_INDENT: f64 = 47.0;
pub const TAG_SIZE_RATIO: f64 = 0.875;
pub const EXTERNAL_ICON: f64 = 14.4;
pub const TAG_PAD: f64 = 4.0;
pub const CODE_SIZE_RATIO: f64 = 0.875; // 14px against a 16px paragraph

/// One run as it will be drawn: text, position, and the style it carries.
pub struct Placed {
    pub text: String,
    pub x: f64,
    pub top: f64,
    pub size: f64,
    pub weight: u16,
    pub italic: bool,
    pub style: Style,
}

pub struct Line {
    pub top: f64,
    pub height: f64,
    pub runs: Vec<Placed>,
}

pub struct Metrics {
    pub heading: Vec<(f64, u16, f64)>, // size, weight, line height
    pub paragraph: (f64, u16, f64),
    /// size, weight, line height, margin below
    pub inline_title: (f64, u16, f64, f64),
}

fn weight_of(style: &Style, base: u16) -> u16 {
    if style.bold { base.max(600) } else { base }
}

/// Lay one block's runs into lines, wrapping at `width`.
fn flow(
    engine: &mut TextEngine,
    runs: &[Run],
    size: f64,
    base_weight: u16,
    line_height: f64,
    x0: f64,
    width: f64,
    top: f64,
) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut current = Line { top, height: line_height, runs: Vec::new() };
    let mut pen = x0;

    for run in runs {
        if run.text == "\n" {
            lines.push(std::mem::replace(
                &mut current,
                Line { top: 0.0, height: line_height, runs: Vec::new() },
            ));
            current.top = lines.last().unwrap().top + line_height;
            pen = x0;
            continue;
        }
        let run_size = if run.style.code {
            size * CODE_SIZE_RATIO
        } else if run.style.tag {
            size * TAG_SIZE_RATIO
        } else {
            size
        };
        let weight = weight_of(&run.style, base_weight);
        // Wrap on words, carrying the run's style across the break. CJK has
        // no spaces, so a break is allowed between any two such characters.
        let mut word_start = 0usize;
        let words: Vec<String> = segments(&run.text);
        let mut pending = String::new();

        for word in words {
            let word = word.as_str();
            let candidate = format!("{pending}{word}");
            let advance = engine.measure_styled(&candidate, run_size as f32, weight, run.style.italic) as f64;
            if pen + advance > x0 + width && !pending.is_empty() {
                current.runs.push(Placed {
                    text: pending.trim_end().to_string(),
                    x: pen,
                    top: current.top,
                    size: run_size,
                    weight,
                    italic: run.style.italic,
                    style: run.style.clone(),
                });
                lines.push(std::mem::replace(
                    &mut current,
                    Line { top: 0.0, height: line_height, runs: Vec::new() },
                ));
                let previous = lines.last().unwrap().top;
                current.top = previous + line_height;
                pen = x0;
                pending = word.to_string();
            } else {
                pending = candidate;
            }
            word_start += word.len();
        }
        let _ = word_start;
        if !pending.is_empty() {
            let advance = engine.measure_styled(&pending, run_size as f32, weight, run.style.italic) as f64;
            let lead = if run.style.tag { TAG_PAD } else { 0.0 };
            current.runs.push(Placed {
                text: pending.clone(),
                x: pen + lead,
                top: current.top,
                size: run_size,
                weight,
                italic: run.style.italic,
                style: run.style.clone(),
            });
            pen += lead + advance
                + if run.style.tag { TAG_PAD } else { 0.0 }
                + if run.style.external { EXTERNAL_ICON } else { 0.0 };
        }
    }
    lines.push(current);
    lines.retain(|l| !l.runs.is_empty());
    lines
}

pub fn lay_out(
    engine: &mut TextEngine,
    blocks: &[Block],
    metrics: &Metrics,
    x0: f64,
    width: f64,
) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let mut cursor = CONTENT_TOP;
    let mut previous_bottom_margin: f64 = 0.0;
    let mut first = true;
    let mut after_title = false;
    // Column positions are shared by every row, so they are measured once
    // over the whole table rather than per row.
    let table_columns = table_layout(engine, blocks, metrics, width);

    for block in blocks {
        let (size, weight, line_height, margin_top, margin_bottom, indent) = match &block.kind {
            Kind::InlineTitle => (
                metrics.inline_title.0,
                metrics.inline_title.1,
                metrics.inline_title.2,
                0.0,
                metrics.inline_title.3,
                0.0,
            ),
            Kind::Heading(level) => {
                let (s, w, lh) = metrics.heading[(*level as usize - 1).min(5)];
                (s, w, lh, 40.0, 0.0, 0.0)
            }
            Kind::Paragraph => {
                let (s, w, lh) = metrics.paragraph;
                (s, w, lh, 16.0, 0.0, 0.0)
            }
            Kind::ListItem { depth, .. } => {
                let (s, w, _) = metrics.paragraph;
                (s, w, LIST_LINE, 0.0, 0.0, LIST_INDENT + *depth as f64 * 36.0)
            }
            Kind::Quote => {
                let (s, w, lh) = metrics.paragraph;
                (s, w, lh, 0.0, 16.0, QUOTE_INDENT)
            }
            Kind::CalloutTitle => {
                let (s, _, lh) = metrics.paragraph;
(s, 600, lh, 12.0, 26.8, CALLOUT_TITLE_INDENT)
            }
            Kind::CalloutBody => {
                let (s, w, lh) = metrics.paragraph;
                (s, w, lh, 0.0, 12.0, CALLOUT_INDENT)
            }
            Kind::Code { .. } => {
                let (s, w, _) = metrics.paragraph;
                (s * CODE_SIZE_RATIO, w, 24.0, 12.0, 12.0, 16.0)
            }
            Kind::TableRow { header, .. } => {
                let (s, w, lh) = metrics.paragraph;
                (s, if *header { 600 } else { w }, lh + 6.0, 0.0, 0.0, 9.0)
            }
            Kind::Rule => {
                let (s, w, _) = metrics.paragraph;
                (s, w, 1.0, 32.0, 32.0, 0.0)
            }
        };

        // The inline title sits outside the content flow: its bottom margin
        // still applies, but the block after it is the first child of the
        // flow and so keeps margin-top zero.
        if !first {
            let top = if after_title { 0.0 } else { margin_top };
            cursor += previous_bottom_margin.max(top);
        }
        after_title = matches!(block.kind, Kind::InlineTitle);
        first = false;
        previous_bottom_margin = margin_bottom;

        match &block.kind {
            Kind::TableRow { cells, header } => {
                let columns = cells.len().max(1);
                let column_width = table_columns
                    .get(0)
                    .copied()
                    .unwrap_or(width / columns as f64);
                let mut line = Line { top: cursor, height: line_height, runs: Vec::new() };
                for (index, cell) in cells.iter().enumerate() {
                    let cell_x = x0 + indent
                        + table_columns.get(index).copied().unwrap_or(index as f64 * column_width);
                    let mut pen = cell_x;
                    for run in cell {
                        let w = if *header { 600 } else { weight_of(&run.style, weight) };
                        let advance = engine.measure_styled(&run.text, size as f32, w, run.style.italic) as f64;
                        line.runs.push(Placed {
                            text: run.text.clone(), x: pen, top: cursor, size,
                            weight: w, italic: run.style.italic, style: run.style.clone(),
                        });
                        pen += advance;
                    }
                }
                cursor += line_height;
                out.push(line);
            }
            Kind::Rule => {
                cursor += line_height;
            }
            Kind::ListItem { marker, depth } => {
                let marker_x = x0 + LIST_INDENT + *depth as f64 * 36.0 - 14.0;
                let mut lines = flow(engine, &block.runs, size, weight, line_height,
                                     x0 + indent, width - indent, cursor);
                if let Some(first_line) = lines.first_mut() {
                    first_line.runs.insert(0, Placed {
                        text: marker.clone(), x: marker_x, top: cursor, size,
                        weight, italic: false,
                        style: Style { marker: true, ..Default::default() },
                    });
                }
                cursor = lines.last().map(|l| l.top + line_height).unwrap_or(cursor);
                out.extend(lines);
            }
            _ => {
                let lines = flow(engine, &block.runs, size, weight, line_height,
                                 x0 + indent, width - indent, cursor);
                cursor = lines.last().map(|l| l.top + line_height).unwrap_or(cursor + line_height);
                out.extend(lines);
            }
        }
    }
    out
}


/// Where each table column starts. Obsidian lets a table size to its content,
/// so the widest cell in a column decides it.
fn table_layout(
    engine: &mut TextEngine,
    blocks: &[Block],
    metrics: &Metrics,
    width: f64,
) -> Vec<f64> {
    const CELL_PAD: f64 = 34.0;
    let mut widest: Vec<f64> = Vec::new();
    for block in blocks {
        if let Kind::TableRow { cells, header } = &block.kind {
            for (index, cell) in cells.iter().enumerate() {
                let text: String = cell.iter().map(|r| r.text.as_str()).collect();
                let weight = if *header { 600 } else { metrics.paragraph.1 };
                let w = engine.measure(&text, metrics.paragraph.0 as f32, weight) as f64 + CELL_PAD;
                if widest.len() <= index {
                    widest.resize(index + 1, 0.0);
                }
                widest[index] = widest[index].max(w);
            }
        }
    }
    let total: f64 = widest.iter().sum();
    if total > width && total > 0.0 {
        let scale = width / total;
        for w in &mut widest {
            *w *= scale;
        }
    }
    let mut starts = Vec::with_capacity(widest.len());
    let mut pen = 0.0;
    for w in widest {
        starts.push(pen);
        pen += w;
    }
    starts
}


/// Break opportunities: after a space, and between adjacent wide characters,
/// which is how CJK wraps since it carries no spaces.
fn segments(text: &str) -> Vec<String> {
    fn wide(c: char) -> bool {
        matches!(c as u32,
            0x1100..=0x11FF | 0x2E80..=0x9FFF | 0xA000..=0xA4CF |
            0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFF60 |
            0x20000..=0x2FA1F)
    }
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut previous: Option<char> = None;
    for c in text.chars() {
        if wide(c) || previous.is_some_and(wide) {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
        }
        current.push(c);
        if c == ' ' {
            out.push(std::mem::take(&mut current));
        }
        previous = Some(c);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// A line is laid right to left when its first strong character is.
pub fn is_rtl(text: &str) -> bool {
    text.chars()
        .find(|c| {
            matches!(*c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
                || c.is_alphabetic()
        })
        .is_some_and(|c| {
            matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
        })
}
