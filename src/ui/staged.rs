use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::{App, PanelFocus};
use crate::git::StatusType;

/// Render the staged files panel.
pub fn draw_staged(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == PanelFocus::Staged;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" STAGED ({}) ", app.staged.len());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.staged.is_empty() {
        let empty = List::new(vec![ListItem::new(Span::styled(
            "  (no staged files)",
            Style::default().fg(Color::DarkGray),
        ))])
        .block(block);
        f.render_widget(empty, area);
        return;
    }

    let selected_idx = app.staged_list_state.selected();

    let items: Vec<ListItem> = app
        .staged
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let is_selected = Some(i) == selected_idx && focused;

            let color = match file.status {
                StatusType::New | StatusType::Added => Color::Green,
                StatusType::Modified => Color::Yellow,
                StatusType::Deleted => Color::Red,
                StatusType::Renamed => Color::Blue,
                StatusType::Copied => Color::Blue,
                StatusType::TypeChange => Color::Magenta,
                StatusType::Untracked => Color::Gray,
                StatusType::Conflicted => Color::Magenta,
            };

            let line = Line::from(vec![
                Span::styled("\u{2713} ", Style::default().fg(color)),
                Span::styled(&file.path, Style::default().fg(color)),
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
