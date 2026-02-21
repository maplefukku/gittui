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

/// Outcomes from background git operations (fetch / push / pull / AI).
#[derive(Debug)]
pub enum AsyncResult {
    FetchComplete(Result<String>),
    PushComplete(Result<String>),
    PullComplete(Result<String>),
    AiCommitMessage(Result<String>),
}

// ---------------------------------------------------------------------------
// EventHandler
// ---------------------------------------------------------------------------

/// Polls crossterm events and converts them to `AppEvent` values.
///
/// The handler uses a configurable tick rate (default 250 ms).  If no
/// terminal event arrives within the tick interval a `Tick` event is
/// produced instead so the main loop can perform periodic housekeeping.
///
/// Also includes a channel for receiving results from background threads.
pub struct EventHandler {
    /// How long to wait before emitting a Tick when no real event arrives.
    tick_rate: Duration,
    async_rx: std::sync::mpsc::Receiver<AsyncResult>,
    async_tx: std::sync::mpsc::Sender<AsyncResult>,
}

impl EventHandler {
    /// Create a new handler with the default 250 ms tick rate.
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tick_rate: Duration::from_millis(250),
            async_rx: rx,
            async_tx: tx,
        }
    }

    /// Create a handler with a custom tick rate.
    pub fn with_tick_rate(tick_rate: Duration) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tick_rate,
            async_rx: rx,
            async_tx: tx,
        }
    }

    /// Get a sender for async results.
    pub fn async_sender(&self) -> std::sync::mpsc::Sender<AsyncResult> {
        self.async_tx.clone()
    }

    /// Block until the next event is available.
    ///
    /// Returns `AppEvent::Tick` if no terminal event arrives within the
    /// configured tick rate. Checks for async results (non-blocking) first.
    pub fn next(&self) -> Result<AppEvent> {
        // Check for async results first (non-blocking)
        if let Ok(result) = self.async_rx.try_recv() {
            return Ok(AppEvent::AsyncResult(result));
        }

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
