use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::app::{App, PanelFocus};
use crate::git::GraphSymbol;

/// Render the commit log with an ASCII graph on the left.
pub fn draw_log_graph(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == PanelFocus::LogGraph;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" LOG ({}) ", app.log.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);

    if app.log.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            " (no commits)",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        f.render_widget(empty, area);
        return;
    }

    let selected_idx = app.log_list_state.selected();

    // Determine if HEAD oid matches a commit to highlight it.
    let head_oid = &app.head.oid;

    let lines: Vec<Line> = app
        .log
        .iter()
        .enumerate()
        .map(|(i, commit)| {
            let is_selected = Some(i) == selected_idx && focused;
            let is_head = !head_oid.is_empty() && commit.oid == *head_oid;

            // -- Graph symbols --------------------------------------------------
            let graph_spans: Vec<Span> = commit
                .graph
                .iter()
                .map(|sym| {
                    let (ch, color) = match sym {
                        GraphSymbol::Commit => ("\u{25cf}", Color::Yellow),     // ●
                        GraphSymbol::Merge => ("\u{25cb}", Color::Magenta),     // ○
                        GraphSymbol::Line | GraphSymbol::Vertical => ("\u{2502}", Color::DarkGray), // │
                        GraphSymbol::Branch => ("/", Color::DarkGray),          // /
                        GraphSymbol::BranchOff => ("\u{251c}", Color::DarkGray),// ├
                        GraphSymbol::Horizontal => ("\u{2500}", Color::DarkGray),// ─
                        GraphSymbol::MergeTop => ("\u{256d}", Color::DarkGray), // ╭
                        GraphSymbol::MergeBottom => ("\u{2570}", Color::DarkGray),// ╰
                        GraphSymbol::Empty | GraphSymbol::Space => (" ", Color::DarkGray),
                    };
                    Span::styled(ch, Style::default().fg(color))
                })
                .collect();

            // -- Short hash -----------------------------------------------------
            let hash_style = if is_head {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            };

            // -- Ref decorations (branches / tags) ------------------------------
            let ref_spans: Vec<Span> = commit
                .refs
                .iter()
                .map(|r| {
                    let color = if r.starts_with("tag:") || r.starts_with("tags/") {
                        Color::Magenta
                    } else if r.contains('/') {
                        // remote branch
                        Color::Red
                    } else {
                        Color::Green
                    };
                    Span::styled(
                        format!(" ({r})"),
                        Style::default()
                            .fg(color)
                            .add_modifier(Modifier::BOLD),
                    )
                })
                .collect();

            // -- Assemble the full line -----------------------------------------
            let mut spans: Vec<Span> = Vec::new();
            spans.extend(graph_spans);
            spans.push(Span::raw(" "));
            spans.push(Span::styled(commit.short_oid.clone(), hash_style));
            spans.push(Span::raw(" "));

            // HEAD marker.
            if is_head {
                spans.push(Span::styled(
                    "(HEAD) ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            spans.push(Span::styled(
                commit.summary.clone(),
                Style::default().fg(Color::White),
            ));
            spans.extend(ref_spans);

            // Author and relative date.
            spans.push(Span::styled(
                format!(" <{}> {}", commit.author, commit.relative_date),
                Style::default().fg(Color::DarkGray),
            ));

            let mut line = Line::from(spans);
            if is_selected {
                line = line.style(Style::default().bg(Color::DarkGray));
            }
            line
        })
        .collect();

    let total_lines = lines.len();
    let visible_height = inner.height as usize;
    let scroll = app.log_viewport_offset.min(total_lines.saturating_sub(visible_height));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((scroll as u16, 0));
    f.render_widget(paragraph, area);

    // -- Scrollbar ----------------------------------------------------------
    if total_lines > visible_height {
        let mut scrollbar_state =
            ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("\u{25b2}"))
            .end_symbol(Some("\u{25bc}"));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
