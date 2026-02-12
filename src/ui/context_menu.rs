use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

/// A single item in a context menu.
pub struct ContextMenuItem<'a> {
    pub label: &'a str,
    pub shortcut: &'a str,
}

/// Render a context menu as a floating popup at the given (x, y) position.
///
/// This is a standalone function that takes explicit parameters rather than
/// relying on app state, since the context menu feature is still a
/// placeholder.
pub fn draw_context_menu(
    f: &mut Frame,
    title: &str,
    items: &[ContextMenuItem<'_>],
    selected: usize,
    x: u16,
    y: u16,
) {
    // Calculate popup dimensions from content.
    let max_label_width = items
        .iter()
        .map(|item| item.label.len() + item.shortcut.len() + 6)
        .max()
        .unwrap_or(20) as u16;

    let width = max_label_width.max(title.len() as u16 + 4).min(60);
    let height = (items.len() as u16 + 2).min(20); // +2 for borders

    let terminal = f.area();

    // Clamp the popup so it stays within the terminal.
    let cx = x.min(terminal.width.saturating_sub(width));
    let cy = y.min(terminal.height.saturating_sub(height));

    let area = Rect::new(cx, cy, width, height);

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == selected;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Right-align the shortcut hint.
            let padding = if width > 4 {
                let used = item.label.len() + item.shortcut.len() + 2;
                let remaining = (width as usize).saturating_sub(used + 4);
                " ".repeat(remaining)
            } else {
                String::new()
            };

            let shortcut_style = if is_selected {
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {}", item.label), style),
                Span::styled(padding, style),
                Span::styled(item.shortcut, shortcut_style),
                Span::styled(" ", style),
            ]))
        })
        .collect();

    let list = List::new(list_items).block(block);
    f.render_widget(list, area);
}
