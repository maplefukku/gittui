use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::{App, PanelFocus};
use crate::git::{FileStatus, StatusType};

/// Render the unstaged / untracked changes panel.
pub fn draw_changes(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == PanelFocus::Changes;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let total = app.changes_len();
    let title = format!(" CHANGES ({total}) ");
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if total == 0 {
        let empty = List::new(vec![ListItem::new(Span::styled(
            "  (working tree clean)",
            Style::default().fg(Color::DarkGray),
        ))])
        .block(block);
        f.render_widget(empty, area);
        return;
    }

    let selected_idx = app.changes_list_state.selected();

    // Build a combined list of unstaged + untracked files.
    let combined: Vec<&FileStatus> = app
        .unstaged
        .iter()
        .chain(app.untracked.iter())
        .collect();

    let items: Vec<ListItem> = combined
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let is_selected = Some(i) == selected_idx && focused;

            let (prefix, color) = match file.status {
                StatusType::Modified => ("M", Color::Yellow),
                StatusType::New | StatusType::Added => ("A", Color::Green),
                StatusType::Deleted => ("D", Color::Red),
                StatusType::Untracked => ("?", Color::Gray),
                StatusType::Conflicted => ("C", Color::Magenta),
                StatusType::Renamed => ("R", Color::Blue),
                StatusType::Copied => ("C", Color::Blue),
                StatusType::TypeChange => ("T", Color::Magenta),
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{prefix} "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(file.path.as_str(), Style::default().fg(color)),
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
