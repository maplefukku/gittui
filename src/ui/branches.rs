use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::{App, PanelFocus};

/// Render the branch list panel.
pub fn draw_branches(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == PanelFocus::Branches;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" BRANCHES ({}) ", app.branches.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.branches.is_empty() {
        let empty = List::new(vec![ListItem::new(Span::styled(
            "  (no branches)",
            Style::default().fg(Color::DarkGray),
        ))])
        .block(block);
        f.render_widget(empty, area);
        return;
    }

    let selected_idx = app.branch_list_state.selected();

    let items: Vec<ListItem> = app
        .branches
        .iter()
        .enumerate()
        .map(|(i, branch)| {
            let is_selected = Some(i) == selected_idx && focused;

            // Prefix: bullet for current branch, whitespace otherwise.
            let prefix = if branch.is_current { "\u{25cf} " } else { "  " };

            // Branch name style.
            let name_style = if branch.is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if branch.is_remote {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default().fg(Color::White)
            };

            // Upstream tracking info.
            let tracking: Vec<Span> = if let Some(ref upstream) = branch.upstream {
                let mut parts = vec![Span::styled(
                    format!(" -> {upstream}"),
                    Style::default().fg(Color::DarkGray),
                )];
                if branch.ahead > 0 {
                    parts.push(Span::styled(
                        format!(" \u{2191}{}", branch.ahead),
                        Style::default().fg(Color::Green),
                    ));
                }
                if branch.behind > 0 {
                    parts.push(Span::styled(
                        format!(" \u{2193}{}", branch.behind),
                        Style::default().fg(Color::Red),
                    ));
                }
                parts
            } else {
                vec![]
            };

            let mut spans = vec![
                Span::styled(prefix, name_style),
                Span::styled(&branch.name, name_style),
            ];
            spans.extend(tracking);

            let mut item = ListItem::new(Line::from(spans));
            if is_selected {
                item = item.style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );
            }
            item
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}
