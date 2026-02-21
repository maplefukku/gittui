//! UI rendering layer for gittui.
//!
//! Each panel lives in its own submodule. The top-level [`draw`] function
//! computes the layout, renders every panel, and then draws any active
//! overlays (help, command palette, dialog, context menu) on top.

pub mod branches;
pub mod changes;
pub mod command_palette;
pub mod commit;
pub mod context_menu;
pub mod dialog;
pub mod diff_viewer;
pub mod help;
pub mod layout;
pub mod log_graph;
pub mod remote;
pub mod staged;
pub mod search;
pub mod stash;
pub mod statusbar;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use layout::LayoutAreas;

/// Main entry point: render the entire application UI for a single frame.
///
/// Call order:
/// 1. Compute layout areas from the terminal size.
/// 2. Draw the header bar.
/// 3. Draw every panel (left sidebar panels + right content panels).
/// 4. Draw the status bar.
/// 5. Draw overlays (help, command palette, dialog, context menu) -- these
///    are rendered last so they float on top of everything else.
pub fn draw(f: &mut Frame, app: &mut App) {
    let areas = LayoutAreas::compute(f.area());

    // -- Header ---------------------------------------------------------
    draw_header(f, &areas, app);

    // -- Left sidebar panels --------------------------------------------
    branches::draw_branches(f, areas.branches, app);
    staged::draw_staged(f, areas.staged, app);
    changes::draw_changes(f, areas.changes, app);
    stash::draw_stash(f, areas.stash, app);
    commit::draw_commit(f, areas.commit, app);
    remote::draw_remote(f, areas.remote, app);

    // -- Right content panels -------------------------------------------
    diff_viewer::draw_diff_viewer(f, areas.diff_viewer, app);
    log_graph::draw_log_graph(f, areas.log_graph, app);

    // -- Search bar (when active) ----------------------------------------
    if app.input_mode == crate::app::InputMode::SearchInput {
        search::draw_search(f, areas.statusbar, app);
    } else {
        // -- Status bar -----------------------------------------------------
        statusbar::draw_statusbar(f, areas.statusbar, app);
    }

    // -- Overlays (drawn last so they are on top) -----------------------
    if app.show_help {
        help::draw_help(f);
    }

    if app.show_command_palette {
        command_palette::draw_command_palette(f, app);
    }

    if let Some(ref dlg) = app.dialog {
        dialog::draw_dialog(f, dlg);
    }

    if let Some(ref menu) = app.context_menu {
        let items: Vec<context_menu::ContextMenuItem<'_>> = menu
            .items
            .iter()
            .map(|item| context_menu::ContextMenuItem {
                label: &item.label,
                shortcut: &item.shortcut,
            })
            .collect();
        context_menu::draw_context_menu(f, &menu.title, &items, menu.selected, menu.x, menu.y);
    }
}

/// Draw the single-line header bar.
///
/// Format: `gittui -- branch_name -- ^N vM`
fn draw_header(f: &mut Frame, areas: &LayoutAreas, app: &App) {
    let logo_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let branch_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let sep_style = Style::default().fg(Color::DarkGray);
    let ahead_style = Style::default().fg(Color::Green);
    let behind_style = Style::default().fg(Color::Red);

    let branch_display = app
        .head
        .branch
        .as_deref()
        .unwrap_or("(detached)");

    let mut spans = vec![
        Span::styled(" gittui", logo_style),
        Span::styled(" \u{2500}\u{2500} ", sep_style),
        Span::styled(branch_display, branch_style),
    ];

    // Show ahead/behind counts when non-zero.
    if app.ahead > 0 || app.behind > 0 {
        spans.push(Span::styled(" \u{2500}\u{2500} ", sep_style));

        if app.ahead > 0 {
            spans.push(Span::styled(
                format!("\u{2191}{}", app.ahead),
                ahead_style,
            ));
        }
        if app.ahead > 0 && app.behind > 0 {
            spans.push(Span::raw(" "));
        }
        if app.behind > 0 {
            spans.push(Span::styled(
                format!("\u{2193}{}", app.behind),
                behind_style,
            ));
        }
    }

    let header_line = Line::from(spans);
    let header = Paragraph::new(header_line)
        .style(Style::default().bg(Color::Black));
    f.render_widget(header, areas.header);
}
