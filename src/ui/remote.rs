use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, PanelFocus};
use crate::git::SyncStatus;

/// Render the remote panel showing remote info and action buttons.
pub fn draw_remote(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == PanelFocus::Remote;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" REMOTE ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // -- Remote info ----------------------------------------------------
    if app.remotes.is_empty() {
        lines.push(Line::from(Span::styled(
            " (no remotes configured)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for remote in &app.remotes {
            let sync_span = match remote.sync_status {
                SyncStatus::UpToDate => Span::styled(
                    " \u{2713} up to date",
                    Style::default().fg(Color::Green),
                ),
                SyncStatus::Ahead(n) => Span::styled(
                    format!(" \u{2191}{n} ahead"),
                    Style::default().fg(Color::Green),
                ),
                SyncStatus::Behind(n) => Span::styled(
                    format!(" \u{2193}{n} behind"),
                    Style::default().fg(Color::Red),
                ),
                SyncStatus::Diverged { ahead, behind } => Span::styled(
                    format!(" \u{2191}{ahead} \u{2193}{behind} diverged"),
                    Style::default().fg(Color::Yellow),
                ),
                SyncStatus::Unknown => Span::styled(
                    " ? unknown",
                    Style::default().fg(Color::DarkGray),
                ),
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {}", remote.name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                sync_span,
            ]));

            // Show URL on a second line if there is space.
            if inner.height > 3 {
                lines.push(Line::from(Span::styled(
                    format!("   {}", remote.url),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    // -- Async operation status -----------------------------------------
    if let Some(ref status) = app.async_status {
        lines.push(Line::from(Span::styled(
            format!(" \u{21bb} {status}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
    }

    // -- Action buttons -------------------------------------------------
    if (inner.height as usize) > lines.len() + 1 {
        // Add a blank separator line before buttons if space allows.
        lines.push(Line::from(""));

        let btn_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(" [Fetch] ", btn_style),
            Span::raw(" "),
            Span::styled(" [Pull] ", btn_style),
            Span::raw(" "),
            Span::styled(" [Push] ", btn_style),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}
