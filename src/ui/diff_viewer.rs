use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::app::{App, DiffMode, PanelFocus};
use crate::git::{DiffLineType, FileDiff};

/// Render the diff viewer panel with syntax-highlighted diff output.
pub fn draw_diff_viewer(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == PanelFocus::DiffViewer;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = match &app.current_diff {
        Some(diff) => format!(" DIFF \u{2014} {} (+{} -{}) ", diff.path, diff.additions, diff.deletions),
        None => " DIFF ".to_string(),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);

    match &app.current_diff {
        None => {
            let placeholder = Paragraph::new(Line::from(Span::styled(
                " Select a file to view its diff",
                Style::default().fg(Color::DarkGray),
            )))
            .block(block);
            f.render_widget(placeholder, area);
        }
        Some(diff) => {
            if diff.is_binary {
                let binary_msg = Paragraph::new(Line::from(Span::styled(
                    " Binary file (no diff available)",
                    Style::default().fg(Color::Yellow),
                )))
                .block(block);
                f.render_widget(binary_msg, area);
                return;
            }

            match app.diff_mode {
                DiffMode::Unified => {
                    let lines = build_diff_lines(diff);
                    let total_lines = lines.len();
                    let visible_height = inner.height as usize;

                    // Clamp scroll offset.
                    let scroll = (app.diff_scroll as usize).min(total_lines.saturating_sub(visible_height));

                    let paragraph = Paragraph::new(lines)
                        .block(block)
                        .scroll((scroll as u16, 0));
                    f.render_widget(paragraph, area);

                    // -- Scrollbar --------------------------------------------------
                    if total_lines > visible_height {
                        let mut scrollbar_state =
                            ScrollbarState::new(total_lines.saturating_sub(visible_height))
                                .position(scroll);
                        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                            .begin_symbol(Some("\u{25b2}"))
                            .end_symbol(Some("\u{25bc}"));
                        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
                    }
                }
                DiffMode::SideBySide => {
                    draw_side_by_side(f, area, block, inner, diff, app.diff_scroll);
                }
            }
        }
    }
}

/// Build the full list of styled `Line`s from a `FileDiff`, including
/// line numbers, hunk headers, and word-level highlighting.
fn build_diff_lines(diff: &FileDiff) -> Vec<Line<'static>> {
    let mut output: Vec<Line<'static>> = Vec::new();

    for hunk in &diff.hunks {
        // -- Hunk header ----------------------------------------------------
        output.push(Line::from(Span::styled(
            hunk.header.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        // Collect additions and deletions in order so we can attempt
        // word-level diff between adjacent deletion/addition pairs.
        let mut i = 0;
        let lines = &hunk.lines;
        while i < lines.len() {
            let line = &lines[i];
            match line.line_type {
                DiffLineType::Context => {
                    let old_no = format_lineno(line.old_lineno);
                    let new_no = format_lineno(line.new_lineno);
                    output.push(Line::from(vec![
                        Span::styled(
                            format!("{old_no} {new_no} "),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!(" {}", line.content),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                    i += 1;
                }
                DiffLineType::Deletion => {
                    // Look ahead for a paired addition to do word-level highlighting.
                    let del_start = i;
                    let mut del_end = i + 1;
                    while del_end < lines.len()
                        && lines[del_end].line_type == DiffLineType::Deletion
                    {
                        del_end += 1;
                    }
                    let add_start = del_end;
                    let mut add_end = add_start;
                    while add_end < lines.len()
                        && lines[add_end].line_type == DiffLineType::Addition
                    {
                        add_end += 1;
                    }

                    let del_count = del_end - del_start;
                    let add_count = add_end - add_start;

                    // Render deletion lines.
                    for d in del_start..del_end {
                        let dl = &lines[d];
                        let old_no = format_lineno(dl.old_lineno);
                        let pair_add = if d - del_start < add_count {
                            Some(&lines[add_start + (d - del_start)])
                        } else {
                            None
                        };

                        let content_spans = if let Some(al) = pair_add {
                            word_highlight_delete(&dl.content, &al.content)
                        } else {
                            vec![Span::styled(
                                dl.content.clone(),
                                Style::default().fg(Color::Red),
                            )]
                        };

                        let mut spans = vec![
                            Span::styled(
                                format!("{old_no}      "),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled("-", Style::default().fg(Color::Red)),
                        ];
                        spans.extend(content_spans);
                        output.push(Line::from(spans));
                    }

                    // Render addition lines.
                    for a in add_start..add_end {
                        let al = &lines[a];
                        let new_no = format_lineno(al.new_lineno);
                        let pair_del = if a - add_start < del_count {
                            Some(&lines[del_start + (a - add_start)])
                        } else {
                            None
                        };

                        let content_spans = if let Some(dl) = pair_del {
                            word_highlight_add(&dl.content, &al.content)
                        } else {
                            vec![Span::styled(
                                al.content.clone(),
                                Style::default().fg(Color::Green),
                            )]
                        };

                        let mut spans = vec![
                            Span::styled(
                                format!("     {new_no} "),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled("+", Style::default().fg(Color::Green)),
                        ];
                        spans.extend(content_spans);
                        output.push(Line::from(spans));
                    }

                    i = add_end;
                }
                DiffLineType::Addition => {
                    // Standalone addition (not paired with a deletion above).
                    let new_no = format_lineno(line.new_lineno);
                    output.push(Line::from(vec![
                        Span::styled(
                            format!("     {new_no} "),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("+{}", line.content),
                            Style::default().fg(Color::Green),
                        ),
                    ]));
                    i += 1;
                }
                DiffLineType::HunkHeader => {
                    output.push(Line::from(Span::styled(
                        line.content.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )));
                    i += 1;
                }
                DiffLineType::Binary => {
                    output.push(Line::from(Span::styled(
                        "Binary file differs",
                        Style::default().fg(Color::Yellow),
                    )));
                    i += 1;
                }
            }
        }
    }

    output
}

/// Format an optional line number into a fixed-width 4-character string.
fn format_lineno(lineno: Option<u32>) -> String {
    match lineno {
        Some(n) => format!("{n:>4}"),
        None => "    ".to_string(),
    }
}

// -- Word-level highlighting ------------------------------------------------

/// Produce spans for a *deleted* line, highlighting the words that differ
/// from the paired addition line.
fn word_highlight_delete(del: &str, add: &str) -> Vec<Span<'static>> {
    word_diff_spans(del, add, Color::Red, Color::LightRed)
}

/// Produce spans for an *added* line, highlighting the words that differ
/// from the paired deletion line.
fn word_highlight_add(del: &str, add: &str) -> Vec<Span<'static>> {
    word_diff_spans(add, del, Color::Green, Color::LightGreen)
}

/// Generic word-level diff highlighting.
///
/// `primary` is the line being rendered, `other` is the line it's compared
/// against. Portions unique to `primary` get `highlight_color`; shared
/// portions get `base_color`.
fn word_diff_spans(
    primary: &str,
    other: &str,
    base_color: Color,
    highlight_color: Color,
) -> Vec<Span<'static>> {
    let p_words: Vec<&str> = primary
        .split_inclusive(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '(' || c == ')')
        .collect();
    let o_words: Vec<&str> = other
        .split_inclusive(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '(' || c == ')')
        .collect();

    // Build a set of words present in `other` for quick lookup.
    let other_set: std::collections::HashSet<&str> = o_words.into_iter().collect();

    let base_style = Style::default().fg(base_color);
    let hl_style = Style::default()
        .fg(highlight_color)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

    let mut spans: Vec<Span<'static>> = Vec::new();

    for word in p_words {
        let style = if other_set.contains(word) {
            base_style
        } else {
            hl_style
        };
        spans.push(Span::styled(word.to_owned(), style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(primary.to_owned(), base_style));
    }

    spans
}

/// Render a side-by-side diff view.
fn draw_side_by_side(
    f: &mut Frame,
    area: Rect,
    block: Block<'_>,
    inner: Rect,
    diff: &FileDiff,
    scroll: u16,
) {
    use ratatui::layout::{Constraint, Direction, Layout};

    f.render_widget(block, area);

    if inner.width < 4 || inner.height == 0 {
        return;
    }

    // Split inner area into left and right halves
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(inner);

    let left_area = halves[0];
    let right_area = halves[1];

    // Build paired lines: (old_line, new_line)
    let mut left_lines: Vec<Line<'static>> = Vec::new();
    let mut right_lines: Vec<Line<'static>> = Vec::new();

    let del_style = Style::default().fg(Color::Red);
    let add_style = Style::default().fg(Color::Green);
    let ctx_style = Style::default().fg(Color::White);
    let hunk_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);

    for hunk in &diff.hunks {
        // Hunk header on both sides
        left_lines.push(Line::from(Span::styled(hunk.header.clone(), hunk_style)));
        right_lines.push(Line::from(Span::styled(hunk.header.clone(), hunk_style)));

        let mut i = 0;
        let lines = &hunk.lines;
        while i < lines.len() {
            match lines[i].line_type {
                DiffLineType::Context => {
                    let content = lines[i].content.clone();
                    left_lines.push(Line::from(Span::styled(format!(" {content}"), ctx_style)));
                    right_lines.push(Line::from(Span::styled(format!(" {content}"), ctx_style)));
                    i += 1;
                }
                DiffLineType::Deletion => {
                    // Collect consecutive deletions
                    let del_start = i;
                    while i < lines.len() && lines[i].line_type == DiffLineType::Deletion {
                        i += 1;
                    }
                    // Collect consecutive additions
                    let add_start = i;
                    while i < lines.len() && lines[i].line_type == DiffLineType::Addition {
                        i += 1;
                    }
                    let del_count = add_start - del_start;
                    let add_count = i - add_start;
                    let max = del_count.max(add_count);

                    for j in 0..max {
                        if j < del_count {
                            let content = &lines[del_start + j].content;
                            left_lines.push(Line::from(Span::styled(format!("-{content}"), del_style)));
                        } else {
                            left_lines.push(Line::from(""));
                        }
                        if j < add_count {
                            let content = &lines[add_start + j].content;
                            right_lines.push(Line::from(Span::styled(format!("+{content}"), add_style)));
                        } else {
                            right_lines.push(Line::from(""));
                        }
                    }
                }
                DiffLineType::Addition => {
                    // Standalone addition (not paired with deletion)
                    let content = lines[i].content.clone();
                    left_lines.push(Line::from(""));
                    right_lines.push(Line::from(Span::styled(format!("+{content}"), add_style)));
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
    }

    let total = left_lines.len();
    let visible = inner.height as usize;
    let scroll_offset = (scroll as usize).min(total.saturating_sub(visible));

    let left_para = Paragraph::new(left_lines).scroll((scroll_offset as u16, 0));
    let right_para = Paragraph::new(right_lines).scroll((scroll_offset as u16, 0));

    f.render_widget(left_para, left_area);
    f.render_widget(right_para, right_area);
}
