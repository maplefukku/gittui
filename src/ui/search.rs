use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

/// Draw the search bar at the bottom of the screen.
pub fn draw_search(f: &mut Frame, area: Rect, app: &App) {
    let result_info = if app.search_results.is_empty() {
        if app.search_query.is_empty() {
            String::new()
        } else {
            " (no results)".to_string()
        }
    } else {
        format!(
            " ({}/{})",
            app.search_result_index + 1,
            app.search_results.len()
        )
    };

    let line = Line::from(vec![
        Span::styled(
            " / ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&app.search_query, Style::default().fg(Color::White)),
        Span::styled(result_info, Style::default().fg(Color::DarkGray)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(line).block(block);
    f.render_widget(paragraph, area);
}
