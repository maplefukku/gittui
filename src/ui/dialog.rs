use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{Dialog, DialogType};

/// Render a centered dialog overlay on top of the existing UI.
pub fn draw_dialog(f: &mut Frame, dialog: &Dialog) {
    let area = f.area();
    let dialog_area = centered_rect(50, 40, area);

    // Clear the region behind the dialog.
    f.render_widget(Clear, dialog_area);

    match &dialog.dialog_type {
        DialogType::Confirm { .. } => draw_confirm_dialog(f, dialog_area, dialog),
        DialogType::Input { .. } => draw_input_dialog(f, dialog_area, dialog),
        DialogType::Select { options, .. } => draw_select_dialog(f, dialog_area, dialog, options),
    }
}

// -- Confirm dialog ---------------------------------------------------------

fn draw_confirm_dialog(f: &mut Frame, area: Rect, dialog: &Dialog) {
    let block = Block::default()
        .title(format!(" {} ", dialog.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 3 || inner.width < 4 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // message
            Constraint::Length(1), // blank
            Constraint::Length(1), // buttons
        ])
        .split(inner);

    // Message.
    let msg = Paragraph::new(dialog.message.as_str())
        .style(Style::default().fg(Color::White));
    f.render_widget(msg, chunks[0]);

    // Buttons.
    let yes_style = if dialog.selected == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let no_style = if dialog.selected == 1 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let buttons = Line::from(vec![
        Span::raw("  "),
        Span::styled(" [Yes] ", yes_style),
        Span::raw("  "),
        Span::styled(" [No] ", no_style),
    ]);
    f.render_widget(Paragraph::new(buttons), chunks[2]);
}

// -- Input dialog -----------------------------------------------------------

fn draw_input_dialog(f: &mut Frame, area: Rect, dialog: &Dialog) {
    let block = Block::default()
        .title(format!(" {} ", dialog.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 3 || inner.width < 4 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // prompt message
            Constraint::Length(1), // blank
            Constraint::Length(1), // input field
        ])
        .split(inner);

    // Prompt.
    let prompt = Paragraph::new(dialog.message.as_str())
        .style(Style::default().fg(Color::White));
    f.render_widget(prompt, chunks[0]);

    // Input field with visible cursor.
    let input_style = Style::default()
        .fg(Color::White)
        .bg(Color::DarkGray);
    let input_text = Paragraph::new(dialog.input.as_str()).style(input_style);
    f.render_widget(input_text, chunks[2]);

    // Position the cursor at the end of input text.
    let cursor_x = chunks[2].x + dialog.input.len() as u16;
    let cursor_y = chunks[2].y;
    if cursor_x < chunks[2].x + chunks[2].width {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

// -- Select dialog ----------------------------------------------------------

fn draw_select_dialog(f: &mut Frame, area: Rect, dialog: &Dialog, options: &[String]) {
    let block = Block::default()
        .title(format!(" {} ", dialog.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 || inner.width < 4 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // message
            Constraint::Min(1),   // option list
        ])
        .split(inner);

    // Message.
    let msg = Paragraph::new(dialog.message.as_str())
        .style(Style::default().fg(Color::White));
    f.render_widget(msg, chunks[0]);

    // Options.
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let style = if i == dialog.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if i == dialog.selected { "> " } else { "  " };
            ListItem::new(Line::from(Span::styled(
                format!("{prefix}{opt}"),
                style,
            )))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, chunks[1]);
}

// -- Helper -----------------------------------------------------------------

/// Return a `Rect` centered within `area`, occupying `percent_x`% of the
/// width and `percent_y`% of the height.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1]);

    horiz[1]
}
