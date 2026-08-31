//! Markdown as Obsidian's reading view renders it.
//!
//! Every constant here was measured off the application, not guessed: see
//! `fixtures/` and `compare.mjs`, which diff this module's layout against what
//! Obsidian produced for the same source.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub tag: bool,
    pub marker: bool,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strike: bool,
    pub mark: bool,
    pub link: bool,
    pub external: bool,
}

#[derive(Clone, Debug)]
pub struct Run {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Debug)]
pub enum Kind {
    /// A file embed shows its filename in this, which is not an h1.
    InlineTitle,
    Heading(u8),
    Paragraph,
    ListItem { depth: usize, marker: String },
    Quote,
    CalloutTitle,
    CalloutBody,
    Code { lang: String },
    TableRow { header: bool, cells: Vec<Vec<Run>> },
    Rule,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub kind: Kind,
    pub runs: Vec<Run>,
}

/// Split one line into styled runs. Obsidian renders `***x***` as an `em`
/// carrying weight 600, so nesting composes rather than replacing.
///
/// Emphasis follows CommonMark flanking: a delimiter opens only when what
/// follows it is not whitespace, and closes only when what precedes it is
/// not whitespace. Without that, `5 * 3 = 15` becomes italic.
pub fn inline(source: &str) -> Vec<Run> {
    let chars: Vec<char> = source.chars().collect();
    let mut runs: Vec<Run> = Vec::new();
    let mut style = Style::default();
    let mut buffer = String::new();
    let mut i = 0;

    fn flush(buffer: &mut String, style: &Style, runs: &mut Vec<Run>) {
        if !buffer.is_empty() {
            runs.push(Run { text: std::mem::take(buffer), style: style.clone() });
        }
    }
    let run_of = |chars: &[char], at: usize, c: char| {
        chars[at..].iter().take_while(|x| **x == c).count()
    };
    let can_open = |chars: &[char], at: usize, len: usize| {
        chars.get(at + len).is_some_and(|c| !c.is_whitespace())
    };
    let can_close = |chars: &[char], at: usize| {
        at > 0 && !chars[at - 1].is_whitespace()
    };

    while i < chars.len() {
        let c = chars[i];

        // escapes: a backslash makes the next character literal
        if c == '\\' {
            if let Some(next) = chars.get(i + 1) {
                buffer.push(*next);
                i += 2;
                continue;
            }
        }

        // code spans: a run of N backticks closes at the next run of N
        if c == '`' {
            let fence = run_of(&chars, i, '`');
            if let Some(close) = (i + fence..chars.len()).find(|&j| {
                chars[j] == '`' && run_of(&chars, j, '`') == fence
            }) {
                flush(&mut buffer, &style, &mut runs);
                let text: String = chars[i + fence..close].iter().collect();
                let mut coded = style.clone();
                coded.code = true;
                runs.push(Run { text: text.trim().to_string(), style: coded });
                i = close + fence;
                continue;
            }
        }

        if c == '*' || c == '_' {
            let len = run_of(&chars, i, c).min(3);
            let opening = can_open(&chars, i, len);
            let closing = can_close(&chars, i);
            // `_` inside a word is a literal, so snake_case survives
            let intraword = c == '_'
                && i > 0
                && chars[i - 1].is_alphanumeric()
                && chars.get(i + len).is_some_and(|n| n.is_alphanumeric());
            let active = if len >= 3 {
                style.bold && style.italic
            } else if len == 2 {
                style.bold
            } else {
                style.italic
            };
            if !intraword && ((opening && !active) || (closing && active)) {
                flush(&mut buffer, &style, &mut runs);
                if len >= 3 {
                    style.bold = !style.bold;
                    style.italic = !style.italic;
                } else if len == 2 {
                    style.bold = !style.bold;
                } else {
                    style.italic = !style.italic;
                }
                i += len;
                continue;
            }
        }

        if c == '~' && run_of(&chars, i, '~') >= 2 {
            flush(&mut buffer, &style, &mut runs);
            style.strike = !style.strike;
            i += 2;
            continue;
        }
        if c == '=' && run_of(&chars, i, '=') >= 2 {
            flush(&mut buffer, &style, &mut runs);
            style.mark = !style.mark;
            i += 2;
            continue;
        }

        // [[wikilink]]
        if c == '[' && chars.get(i + 1) == Some(&'[') {
            if let Some(close) = (i..chars.len()).find(|&j| {
                chars[j] == ']' && chars.get(j + 1) == Some(&']')
            }) {
                flush(&mut buffer, &style, &mut runs);
                let target: String = chars[i + 2..close].iter().collect();
                let shown = target.rsplit('|').next().unwrap_or(&target).to_string();
                let mut linked = style.clone();
                linked.link = true;
                runs.push(Run { text: shown, style: linked });
                i = close + 2;
                continue;
            }
        }

        // [text](target) — the text is itself markdown
        if c == '[' {
            if let Some(close) = (i..chars.len()).find(|&j| chars[j] == ']') {
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(end) = (close..chars.len()).find(|&j| chars[j] == ')') {
                        flush(&mut buffer, &style, &mut runs);
                        let label: String = chars[i + 1..close].iter().collect();
                        for mut inner in inline(&label) {
                            inner.style.link = true;
                            inner.style.external = true;
                            inner.style.bold |= style.bold;
                            inner.style.italic |= style.italic;
                            runs.push(inner);
                        }
                        i = end + 1;
                        continue;
                    }
                }
            }
        }

        // #tag
        if c == '#'
            && (i == 0 || chars[i - 1].is_whitespace())
            && chars.get(i + 1).is_some_and(|n| n.is_alphanumeric())
        {
            flush(&mut buffer, &style, &mut runs);
            let end = chars[i..]
                .iter()
                .position(|x| x.is_whitespace())
                .map(|p| i + p)
                .unwrap_or(chars.len());
            let mut tagged = style.clone();
            tagged.tag = true;
            runs.push(Run { text: chars[i..end].iter().collect(), style: tagged });
            i = end;
            continue;
        }

        buffer.push(c);
        i += 1;
    }
    flush(&mut buffer, &style, &mut runs);
    runs.retain(|r| !r.text.is_empty());
    runs
}

fn indent_depth(line: &str) -> usize {
    let spaces = line.len() - line.trim_start().len();
    spaces / 2
}

pub fn parse_blocks(source: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut fence: Option<(String, Vec<String>)> = None;
    let mut table: Vec<Vec<String>> = Vec::new();

    /// Consecutive lines are one paragraph joined by hard breaks, not one
    /// block each: a `<br>` costs a line box, a new block costs a margin too.
    fn flush_paragraph(paragraph: &mut Vec<String>, blocks: &mut Vec<Block>) {
        if paragraph.is_empty() {
            return;
        }
        let mut runs = Vec::new();
        for (index, line) in paragraph.drain(..).enumerate() {
            if index > 0 {
                runs.push(Run { text: "\n".into(), style: Style::default() });
            }
            runs.extend(inline(&line));
        }
        blocks.push(Block { kind: Kind::Paragraph, runs });
    }
    fn flush_table(table: &mut Vec<Vec<String>>, blocks: &mut Vec<Block>) {
        for (index, row) in table.drain(..).enumerate() {
            let cells: Vec<Vec<Run>> = row.iter().map(|c| inline(c)).collect();
            blocks.push(Block {
                kind: Kind::TableRow { header: index == 0, cells },
                runs: Vec::new(),
            });
        }
    }

    for raw in source.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim_start();

        if let Some((lang, body)) = fence.as_mut() {
            if trimmed.starts_with("```") {
                blocks.push(Block {
                    kind: Kind::Code { lang: lang.clone() },
                    runs: body.iter().map(|l| Run { text: l.clone(), style: Style { code: true, ..Default::default() } }).collect(),
                });
                fence = None;
            } else {
                body.push(line.to_string());
            }
            continue;
        }
        if let Some(lang) = trimmed.strip_prefix("```") {
            flush_paragraph(&mut paragraph, &mut blocks);
            fence = Some((lang.trim().to_string(), Vec::new()));
            continue;
        }

        // table rows accumulate; the delimiter row is dropped
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let cells: Vec<String> = trimmed
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect();
            if cells.iter().all(|c| c.chars().all(|ch| ch == '-' || ch == ':') && !c.is_empty()) {
                continue;
            }
            flush_paragraph(&mut paragraph, &mut blocks);
            table.push(cells);
            continue;
        }
        flush_table(&mut table, &mut blocks);

        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            flush_paragraph(&mut paragraph, &mut blocks);
            blocks.push(Block { kind: Kind::Rule, runs: Vec::new() });
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('#') {
            let extra = rest.chars().take_while(|c| *c == '#').count();
            let level = 1 + extra as u8;
            let body = rest[extra..].trim_start();
            if level <= 6 && rest.chars().nth(extra) == Some(' ') && !body.is_empty() {
                flush_paragraph(&mut paragraph, &mut blocks);
                blocks.push(Block { kind: Kind::Heading(level), runs: inline(body) });
                continue;
            }
        }

        if let Some(quoted) = trimmed.strip_prefix('>') {
            flush_paragraph(&mut paragraph, &mut blocks);
            let body = quoted.trim_start();
            if let Some(rest) = body.strip_prefix("[!") {
                let title = rest.splitn(2, ']').nth(1).unwrap_or("").trim();
                blocks.push(Block { kind: Kind::CalloutTitle, runs: inline(title) });
            } else {
                let previous_callout = matches!(
                    blocks.last().map(|b| &b.kind),
                    Some(Kind::CalloutTitle) | Some(Kind::CalloutBody)
                );
                let kind = if previous_callout { Kind::CalloutBody } else { Kind::Quote };
                // A run of quoted lines is one block; a hard break inside it
                // costs a line box but no margin.
                let continues = matches!(blocks.last().map(|b| &b.kind), Some(Kind::Quote))
                    && matches!(kind, Kind::Quote);
                if continues {
                    let last = blocks.last_mut().unwrap();
                    last.runs.push(Run { text: "\n".into(), style: Style::default() });
                    last.runs.extend(inline(body));
                } else {
                    blocks.push(Block { kind, runs: inline(body) });
                }
            }
            continue;
        }

        let bullet = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "));
        let ordered = trimmed
            .split_once(". ")
            .filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));

        if let Some(item) = bullet {
            flush_paragraph(&mut paragraph, &mut blocks);
            let (marker, body) = match item.strip_prefix("[ ] ") {
                Some(rest) => ("☐".to_string(), rest),
                None => match item.strip_prefix("[x] ") {
                    Some(rest) => ("☑".to_string(), rest),
                    None => ("•".to_string(), item),
                },
            };
            blocks.push(Block {
                kind: Kind::ListItem { depth: indent_depth(line), marker },
                runs: inline(body),
            });
            continue;
        }
        if let Some((number, item)) = ordered {
            flush_paragraph(&mut paragraph, &mut blocks);
            blocks.push(Block {
                kind: Kind::ListItem { depth: indent_depth(line), marker: format!("{number}.") },
                runs: inline(item),
            });
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut paragraph, &mut blocks);
        } else {
            paragraph.push(trimmed.to_string());
        }
    }
    flush_paragraph(&mut paragraph, &mut blocks);
    flush_table(&mut table, &mut blocks);
    if let Some((lang, body)) = fence {
        blocks.push(Block {
            kind: Kind::Code { lang },
            runs: body.iter().map(|l| Run { text: l.clone(), style: Style { code: true, ..Default::default() } }).collect(),
        });
    }
    blocks
}
