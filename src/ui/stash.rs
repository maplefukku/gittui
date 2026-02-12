use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::{App, PanelFocus};

/// Render the stash list panel.
pub fn draw_stash(f: &mut Frame, area: Rect, app: &App) {
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

    let selected_idx = app.stash_list_state.selected();

    let items: Vec<ListItem> = app
        .stashes
        .iter()
        .enumerate()
        .map(|(i, stash)| {
            let is_selected = Some(i) == selected_idx && focused;

            let line = Line::from(vec![
                Span::styled(
                    format!("stash@{{{}}} ", stash.index),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(&stash.message, Style::default().fg(Color::White)),
            ]);

            let mut item = ListItem::new(line);
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
