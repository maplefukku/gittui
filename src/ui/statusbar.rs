use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;

/// Render the status bar at the bottom of the screen.
pub fn draw_statusbar(f: &mut Frame, area: Rect, app: &App) {
    let key_style = Style::default()
        .fg(Color::Black)
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Color::Gray);

    let mut spans = vec![
        Span::styled(" [?] ", key_style),
        Span::styled("Help ", label_style),
        Span::styled(" [q] ", key_style),
        Span::styled("Quit ", label_style),
        Span::styled(" [/] ", key_style),
        Span::styled("Search ", label_style),
        Span::styled(" [r] ", key_style),
        Span::styled("Refresh ", label_style),
    ];

    // -- Repo status --------------------------------------------------------
    let is_clean = app.staged.is_empty()
        && app.unstaged.is_empty()
        && app.untracked.is_empty()
        && app.conflicts.is_empty();

    let status_msg = if is_clean {
        Span::styled(
            "\u{2713} clean",
            Style::default().fg(Color::Green),
        )
    } else {
        Span::styled(
            "\u{2717} dirty",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    };

    spans.push(Span::raw("  \u{2502} "));
    spans.push(status_msg);

    // -- Notification -------------------------------------------------------
    if let Some(ref notif) = app.notification {
        let notif_style = if notif.is_error {
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        spans.push(Span::raw("  \u{2502} "));
        spans.push(Span::styled(notif.message.as_str(), notif_style));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line)
        .style(Style::default().bg(Color::Black));
    f.render_widget(paragraph, area);
}
