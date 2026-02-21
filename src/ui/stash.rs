use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::{App, PanelFocus};

/// Render the stash list panel.
pub fn draw_stash(f: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.focus == PanelFocus::Stash;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" STASH ({}) ", app.stashes.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.stashes.is_empty() {
        let empty = List::new(vec![ListItem::new(Span::styled(
            "  (no stashes)",
            Style::default().fg(Color::DarkGray),
        ))])
        .block(block);
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .stashes
        .iter()
        .map(|stash| {
            let line = Line::from(vec![
                Span::styled(
                    format!("stash@{{{}}} ", stash.index),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(&stash.message, Style::default().fg(Color::White)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(list, area, &mut app.stash_list_state);
}
