use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, InputMode, PanelFocus};

/// Render the commit message input panel.
pub fn draw_commit(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == PanelFocus::CommitMessage;
    let editing = app.input_mode == InputMode::EditingCommitMessage;

    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if app.amend_mode {
        " COMMIT (AMEND) "
    } else {
        " COMMIT "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    // Split the commit area into the text input region and a button bar.
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // text input area
            Constraint::Length(1), // button bar
        ])
        .split(inner);

    let text_area = chunks[0];
    let button_area = chunks[1];

    // -- Text input -----------------------------------------------------
    let msg_style = if editing {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::Gray)
    };

    let display_text = if app.commit_message.is_empty() && !editing {
        "Enter commit message..."
    } else {
        &app.commit_message
    };

    let paragraph = Paragraph::new(display_text).style(msg_style);
    f.render_widget(paragraph, text_area);

    // Show cursor when editing.
    if editing {
        let cursor_x = text_area.x + app.commit_cursor as u16;
        let cursor_y = text_area.y;
        // Clamp so the cursor stays inside the visible area.
        if cursor_x < text_area.x + text_area.width {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    // -- Button bar -----------------------------------------------------
    let commit_btn_style = if focused && !app.amend_mode {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let amend_btn_style = if focused && app.amend_mode {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let buttons = Line::from(vec![
        Span::styled(" [Commit] ", commit_btn_style),
        Span::raw(" "),
        Span::styled(" [Amend] ", amend_btn_style),
    ]);

    let buttons_paragraph = Paragraph::new(buttons);
    f.render_widget(buttons_paragraph, button_area);
}
