use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, MouseEvent};

// ---------------------------------------------------------------------------
// AppEvent
// ---------------------------------------------------------------------------

/// High-level events consumed by the application loop.
#[derive(Debug)]
pub enum AppEvent {
    /// A keyboard event.
    Key(KeyEvent),
    /// A mouse event.
    Mouse(MouseEvent),
    /// Periodic tick (used for animations, notification expiry, etc.).
    Tick,
    /// Terminal was resized.
    Resize(u16, u16),
    /// An async operation completed.
    AsyncResult(AsyncResult),
}

/// Outcomes from background git operations (fetch / push / pull).
#[derive(Debug)]
pub enum AsyncResult {
    FetchComplete(Result<String>),
    PushComplete(Result<String>),
    PullComplete(Result<String>),
}

// ---------------------------------------------------------------------------
// EventHandler
// ---------------------------------------------------------------------------

/// Polls crossterm events and converts them to `AppEvent` values.
///
/// The handler uses a configurable tick rate (default 250 ms).  If no
/// terminal event arrives within the tick interval a `Tick` event is
/// produced instead so the main loop can perform periodic housekeeping.
pub struct EventHandler {
    /// How long to wait before emitting a Tick when no real event arrives.
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new handler with the default 250 ms tick rate.
    pub fn new() -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
        }
    }

    /// Create a handler with a custom tick rate.
    pub fn with_tick_rate(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Block until the next event is available.
    ///
    /// Returns `AppEvent::Tick` if no terminal event arrives within the
    /// configured tick rate.
    pub fn next(&self) -> Result<AppEvent> {
        if event::poll(self.tick_rate)? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) => Ok(AppEvent::Key(key)),
                Event::Mouse(mouse) => Ok(AppEvent::Mouse(mouse)),
                Event::Resize(w, h) => Ok(AppEvent::Resize(w, h)),
                // FocusGained, FocusLost, Paste – treat as ticks
                _ => Ok(AppEvent::Tick),
            }
        } else {
            Ok(AppEvent::Tick)
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
