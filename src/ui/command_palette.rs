use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

/// Default list of commands shown in the palette.
const COMMANDS: &[(&str, &str)] = &[
    ("Commit", "c"),
    ("Amend", "C"),
    ("Stage File", "s"),
    ("Unstage File", "u"),
    ("Stage All", "S"),
    ("Unstage All", "U"),
    ("Fetch", "f"),
    ("Pull", "p"),
    ("Push", "P"),
    ("Create Branch", "b"),
    ("Delete Branch", "D"),
    ("Stash", "z"),
    ("Pop Stash", "Z"),
    ("Toggle Diff Mode", "d"),
    ("Refresh", "r"),
    ("Quit", "q"),
    ("Help", "?"),
    ("Search", "/"),
];

/// Render a VS Code-style command palette overlay.
pub fn draw_command_palette(f: &mut Frame, app: &App) {
    let area = f.area();
    let palette_area = palette_rect(area);

    f.render_widget(Clear, palette_area);

    let block = Block::default()
        .title(" Command Palette ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(palette_area);
    f.render_widget(block, palette_area);

    if inner.height < 3 || inner.width < 8 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // input field
            Constraint::Length(1), // separator
            Constraint::Min(1),   // filtered results list
        ])
        .split(inner);

    // -- Input field --------------------------------------------------------
    let input_prefix = "> ";
    let input_line = Line::from(vec![
        Span::styled(
            input_prefix,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.command_palette_input.as_str(),
            Style::default().fg(Color::White),
        ),
    ]);

    let input_paragraph = Paragraph::new(input_line);
    f.render_widget(input_paragraph, chunks[0]);

    // Position cursor.
    let cursor_x = chunks[0].x + input_prefix.len() as u16 + app.command_palette_input.len() as u16;
    let cursor_y = chunks[0].y;
    if cursor_x < chunks[0].x + chunks[0].width {
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // -- Separator ----------------------------------------------------------
    let sep = Paragraph::new(Line::from(Span::styled(
        "\u{2500}".repeat(chunks[1].width as usize),
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(sep, chunks[1]);

    // -- Filtered command list ----------------------------------------------
    let filtered = filter_commands(&app.command_palette_input);
    let selected = app.command_palette_selected;

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, (name, shortcut))| {
            let is_selected = i == selected;

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let shortcut_style = if is_selected {
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let bg_style = if is_selected {
                Style::default().bg(Color::Cyan)
            } else {
                Style::default()
            };

            // Right-align shortcut: compute padding.
            let avail = chunks[2].width as usize;
            let used = name.len() + shortcut.len() + 4; // 2 prefix + 2 spacing
            let pad = avail.saturating_sub(used);

            ListItem::new(Line::from(vec![
                Span::styled("  ", bg_style),
                Span::styled(*name, name_style),
                Span::styled(" ".repeat(pad), bg_style),
                Span::styled(*shortcut, shortcut_style),
                Span::styled("  ", bg_style),
            ]))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, chunks[2]);
}

/// Filter commands using simple case-insensitive substring matching.
fn filter_commands(query: &str) -> Vec<(&'static str, &'static str)> {
    if query.is_empty() {
        return COMMANDS.to_vec();
    }

    let query_lower = query.to_lowercase();
    COMMANDS
        .iter()
        .filter(|(name, _)| name.to_lowercase().contains(&query_lower))
        .copied()
        .collect()
}

/// Compute the palette rect: centered horizontally, anchored near the top
/// of the screen (like VS Code's Ctrl+P).
fn palette_rect(area: Rect) -> Rect {
    let width = (area.width * 60 / 100).clamp(30, 80);
    let height = (area.height * 50 / 100).clamp(6, 20);
    let x = area.width.saturating_sub(width) / 2;
    let y = area.height / 6; // roughly 1/6 from the top

    Rect::new(x, y, width, height)
}
