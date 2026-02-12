use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Render a full-screen help overlay showing all keybindings by category.
pub fn draw_help(f: &mut Frame) {
    let area = f.area();

    // The overlay occupies 80% x 80% of the terminal, centered.
    let overlay = centered_rect(80, 80, area);

    f.render_widget(Clear, overlay);

    let block = Block::default()
        .title(" Help \u{2014} Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(overlay);
    f.render_widget(block, overlay);

    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);
    let heading_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

    let lines: Vec<Line> = vec![
        // ── Navigation ────────────────────────────────────────────────
        Line::from(Span::styled("Navigation", heading_style)),
        Line::from(""),
        help_line("j / \u{2193}", "Move down", key_style, desc_style),
        help_line("k / \u{2191}", "Move up", key_style, desc_style),
        help_line("Tab", "Next panel", key_style, desc_style),
        help_line("Shift+Tab", "Previous panel", key_style, desc_style),
        help_line("1\u{2013}8", "Jump to panel by number", key_style, desc_style),
        help_line("g / Home", "Go to top of list", key_style, desc_style),
        help_line("G / End", "Go to bottom of list", key_style, desc_style),
        help_line("Ctrl+d", "Half-page down", key_style, desc_style),
        help_line("Ctrl+u", "Half-page up", key_style, desc_style),
        Line::from(""),
        // ── Staging ───────────────────────────────────────────────────
        Line::from(Span::styled("Staging", heading_style)),
        Line::from(""),
        help_line("s", "Stage file under cursor", key_style, desc_style),
        help_line("u", "Unstage file under cursor", key_style, desc_style),
        help_line("S", "Stage all changes", key_style, desc_style),
        help_line("U", "Unstage all files", key_style, desc_style),
        Line::from(""),
        // ── Committing ────────────────────────────────────────────────
        Line::from(Span::styled("Committing", heading_style)),
        Line::from(""),
        help_line("c", "Open commit message editor", key_style, desc_style),
        help_line("C", "Amend last commit", key_style, desc_style),
        help_line("Enter", "Submit commit (when editing)", key_style, desc_style),
        help_line("Escape", "Cancel editing / close overlay", key_style, desc_style),
        Line::from(""),
        // ── Branches ──────────────────────────────────────────────────
        Line::from(Span::styled("Branches", heading_style)),
        Line::from(""),
        help_line("b", "Create new branch", key_style, desc_style),
        help_line("B", "Rename branch", key_style, desc_style),
        help_line("D", "Delete branch", key_style, desc_style),
        help_line("Enter", "Checkout branch", key_style, desc_style),
        Line::from(""),
        // ── Remote ────────────────────────────────────────────────────
        Line::from(Span::styled("Remote", heading_style)),
        Line::from(""),
        help_line("f", "Fetch", key_style, desc_style),
        help_line("p", "Pull", key_style, desc_style),
        help_line("P", "Push", key_style, desc_style),
        Line::from(""),
        // ── Stash ─────────────────────────────────────────────────────
        Line::from(Span::styled("Stash", heading_style)),
        Line::from(""),
        help_line("z", "Stash changes", key_style, desc_style),
        help_line("Z", "Pop stash", key_style, desc_style),
        Line::from(""),
        // ── Diff ──────────────────────────────────────────────────────
        Line::from(Span::styled("Diff", heading_style)),
        Line::from(""),
        help_line("d", "Toggle diff mode (unified/split)", key_style, desc_style),
        help_line("Enter", "Expand/collapse hunk", key_style, desc_style),
        Line::from(""),
        // ── General ───────────────────────────────────────────────────
        Line::from(Span::styled("General", heading_style)),
        Line::from(""),
        help_line("?", "Toggle this help", key_style, desc_style),
        help_line("q", "Quit", key_style, desc_style),
        help_line("/", "Search", key_style, desc_style),
        help_line("r", "Refresh", key_style, desc_style),
        help_line("Ctrl+p", "Command palette", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled(
            "Press Escape or ? to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

/// Build a single help line: `  key          description`.
fn help_line<'a>(
    key: &'a str,
    desc: &'a str,
    key_style: Style,
    desc_style: Style,
) -> Line<'a> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{key:<16}"), key_style),
        Span::styled(desc, desc_style),
    ])
}

/// Return a `Rect` centered within `area`.
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
