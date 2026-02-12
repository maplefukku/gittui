pub mod app;
pub mod config;
pub mod event;
pub mod git;
pub mod input;
pub mod ui;

use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;
use event::{AppEvent, EventHandler};
use input::mouse::PanelLayout;
use ui::layout::LayoutAreas;

fn main() -> Result<()> {
    // Determine the repository path: use the first CLI argument or CWD.
    let repo_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Initialise the application state.
    let mut app = App::new(repo_path).context("failed to initialise gittui")?;

    // Set up the terminal for TUI rendering.
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to create terminal")?;

    // Build the event handler (250 ms tick rate).
    let events = EventHandler::new();

    // Track panel layout for mouse hit-testing.
    let mut panel_layout = PanelLayout::default();

    // ── Main event loop ──────────────────────────────────────────────────
    let result = run_loop(&mut terminal, &mut app, &events, &mut panel_layout);

    // ── Restore the terminal ─────────────────────────────────────────────
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )
    .context("failed to leave alternate screen")?;
    terminal.show_cursor().context("failed to show cursor")?;

    // Propagate any error from the main loop.
    result
}

/// The main event loop: render → poll → dispatch → repeat.
fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    events: &EventHandler,
    panel_layout: &mut PanelLayout,
) -> Result<()> {
    loop {
        // ── Render ────────────────────────────────────────────────────
        terminal.draw(|frame| {
            // Compute layout once per frame so mouse handler can reuse it.
            let areas = LayoutAreas::compute(frame.area());

            // Store rects for mouse hit-testing.
            panel_layout.branches = areas.branches;
            panel_layout.staged = areas.staged;
            panel_layout.changes = areas.changes;
            panel_layout.stash = areas.stash;
            panel_layout.commit_message = areas.commit;
            panel_layout.remote = areas.remote;
            panel_layout.diff_viewer = areas.diff_viewer;
            panel_layout.log_graph = areas.log_graph;

            ui::draw(frame, app);
        })?;

        if app.should_quit {
            return Ok(());
        }

        // ── Handle events ─────────────────────────────────────────────
        match events.next()? {
            AppEvent::Key(key) => {
                input::handle_key_event(app, key)?;
            }
            AppEvent::Mouse(mouse) => {
                input::handle_mouse_event(app, mouse, Some(panel_layout))?;
            }
            AppEvent::Tick => {
                app.tick_notification();
            }
            AppEvent::Resize(_w, _h) => {
                // Terminal resize is handled automatically by ratatui on next draw.
            }
            AppEvent::AsyncResult(_result) => {
                // Async results will be handled when we integrate tokio.
                app.async_status = None;
                app.refresh().ok();
            }
        }
    }
}
