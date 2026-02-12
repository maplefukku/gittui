use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A minimal text editor for the commit message area.
///
/// The editor operates on a `String` and a cursor position (byte offset).
/// It supports:
/// - Character insertion at cursor
/// - Backspace / Delete
/// - Left / Right arrow cursor movement
/// - Home / End
/// - Multi-line editing with Enter
/// - Ctrl+A (select all / home), Ctrl+E (end)
///
/// Process a single key event against the buffer and cursor.
pub fn handle_text_input(buf: &mut String, cursor: &mut usize, key: KeyEvent) {
    // Ensure cursor is in bounds after any external mutation.
    if *cursor > buf.len() {
        *cursor = buf.len();
    }

    match key.code {
        // ── Character insertion ────────────────────────────────────────
        KeyCode::Char(c) => {
            // Ctrl+A -> move to start of current line
            if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'a' {
                *cursor = line_start(buf, *cursor);
                return;
            }
            // Ctrl+E -> move to end of current line
            if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'e' {
                *cursor = line_end(buf, *cursor);
                return;
            }
            // Ctrl+K -> delete from cursor to end of line
            if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'k' {
                let end = line_end(buf, *cursor);
                if end == *cursor && end < buf.len() {
                    // At end of line, delete the newline character.
                    buf.remove(end);
                } else {
                    buf.drain(*cursor..end);
                }
                return;
            }
            // Ctrl+U -> delete from start of line to cursor
            if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'u' {
                let start = line_start(buf, *cursor);
                buf.drain(start..*cursor);
                *cursor = start;
                return;
            }
            // Regular character insertion.
            buf.insert(*cursor, c);
            *cursor += c.len_utf8();
        }

        // ── Enter (new line) ──────────────────────────────────────────
        KeyCode::Enter => {
            buf.insert(*cursor, '\n');
            *cursor += 1;
        }

        // ── Backspace ─────────────────────────────────────────────────
        KeyCode::Backspace => {
            if *cursor > 0 {
                // Find the previous character boundary.
                let prev = prev_char_boundary(buf, *cursor);
                buf.drain(prev..*cursor);
                *cursor = prev;
            }
        }

        // ── Delete ────────────────────────────────────────────────────
        KeyCode::Delete => {
            if *cursor < buf.len() {
                let next = next_char_boundary(buf, *cursor);
                buf.drain(*cursor..next);
            }
        }

        // ── Left ──────────────────────────────────────────────────────
        KeyCode::Left => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Move to the start of the previous word.
                *cursor = prev_word_boundary(buf, *cursor);
            } else {
                *cursor = prev_char_boundary(buf, *cursor);
            }
        }

        // ── Right ─────────────────────────────────────────────────────
        KeyCode::Right => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Move to the start of the next word.
                *cursor = next_word_boundary(buf, *cursor);
            } else if *cursor < buf.len() {
                *cursor = next_char_boundary(buf, *cursor);
            }
        }

        // ── Home ──────────────────────────────────────────────────────
        KeyCode::Home => {
            *cursor = line_start(buf, *cursor);
        }

        // ── End ───────────────────────────────────────────────────────
        KeyCode::End => {
            *cursor = line_end(buf, *cursor);
        }

        // ── Up arrow (move to same column on previous line) ───────────
        KeyCode::Up => {
            let (line_idx, col) = cursor_line_col(buf, *cursor);
            if line_idx > 0 {
                *cursor = line_col_to_offset(buf, line_idx - 1, col);
            }
        }

        // ── Down arrow (move to same column on next line) ─────────────
        KeyCode::Down => {
            let (line_idx, col) = cursor_line_col(buf, *cursor);
            let line_count = buf.matches('\n').count() + 1;
            if line_idx + 1 < line_count {
                *cursor = line_col_to_offset(buf, line_idx + 1, col);
            }
        }

        _ => {}
    }
}

/// A `TextEditor` wraps a buffer + cursor for convenient use.
pub struct TextEditor {
    pub buffer: String,
    pub cursor: usize,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
        }
    }

    pub fn with_text(text: &str) -> Self {
        let len = text.len();
        Self {
            buffer: text.to_string(),
            cursor: len,
        }
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        handle_text_input(&mut self.buffer, &mut self.cursor, key);
    }

    pub fn text(&self) -> &str {
        &self.buffer
    }

    pub fn set_text(&mut self, text: &str) {
        self.buffer = text.to_string();
        self.cursor = self.buffer.len();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    /// Return (line_index, column) of the cursor.
    pub fn cursor_position(&self) -> (usize, usize) {
        cursor_line_col(&self.buffer, self.cursor)
    }

    /// Return the number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.buffer.matches('\n').count() + 1
    }
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Find the byte offset of the start of the line containing `pos`.
fn line_start(buf: &str, pos: usize) -> usize {
    buf[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

/// Find the byte offset of the end of the line containing `pos` (just
/// before the newline, or the end of the buffer).
fn line_end(buf: &str, pos: usize) -> usize {
    buf[pos..]
        .find('\n')
        .map(|i| pos + i)
        .unwrap_or(buf.len())
}

/// Move backward one char boundary.
fn prev_char_boundary(buf: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut p = pos - 1;
    while p > 0 && !buf.is_char_boundary(p) {
        p -= 1;
    }
    p
}

/// Move forward one char boundary.
fn next_char_boundary(buf: &str, pos: usize) -> usize {
    if pos >= buf.len() {
        return buf.len();
    }
    let mut p = pos + 1;
    while p < buf.len() && !buf.is_char_boundary(p) {
        p += 1;
    }
    p
}

/// Move backward to the start of the previous word.
fn prev_word_boundary(buf: &str, pos: usize) -> usize {
    let bytes = buf.as_bytes();
    let mut p = pos;

    // Skip whitespace going backward.
    while p > 0 && (bytes[p - 1] as char).is_whitespace() {
        p -= 1;
    }
    // Skip word characters going backward.
    while p > 0 && !(bytes[p - 1] as char).is_whitespace() {
        p -= 1;
    }
    p
}

/// Move forward to the end of the next word.
fn next_word_boundary(buf: &str, pos: usize) -> usize {
    let bytes = buf.as_bytes();
    let len = buf.len();
    let mut p = pos;

    // Skip current word characters.
    while p < len && !(bytes[p] as char).is_whitespace() {
        p += 1;
    }
    // Skip whitespace.
    while p < len && (bytes[p] as char).is_whitespace() {
        p += 1;
    }
    p
}

/// Convert a byte offset to (line_index, column).
fn cursor_line_col(buf: &str, pos: usize) -> (usize, usize) {
    let before = &buf[..pos.min(buf.len())];
    let line_idx = before.matches('\n').count();
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = pos - line_start;
    (line_idx, col)
}

/// Convert (line_index, column) to a byte offset, clamping the column to
/// the line length.
fn line_col_to_offset(buf: &str, target_line: usize, target_col: usize) -> usize {
    let mut current_line = 0;
    let mut line_start = 0;

    for (i, ch) in buf.char_indices() {
        if current_line == target_line {
            let col = i - line_start;
            if col == target_col {
                return i;
            }
            if ch == '\n' {
                // Reached end of the target line; clamp.
                return i;
            }
        } else if ch == '\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }

    // If we're on the target line and ran out of buffer, clamp to end.
    if current_line == target_line {
        let remaining_col = buf.len() - line_start;
        return line_start + remaining_col.min(target_col);
    }

    buf.len()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_insert_chars() {
        let mut buf = String::new();
        let mut cursor = 0;
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Char('h')));
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Char('i')));
        assert_eq!(buf, "hi");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn test_backspace() {
        let mut buf = "hello".to_string();
        let mut cursor = 5;
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Backspace));
        assert_eq!(buf, "hell");
        assert_eq!(cursor, 4);
    }

    #[test]
    fn test_delete() {
        let mut buf = "hello".to_string();
        let mut cursor = 0;
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Delete));
        assert_eq!(buf, "ello");
        assert_eq!(cursor, 0);
    }

    #[test]
    fn test_left_right() {
        let mut buf = "abc".to_string();
        let mut cursor = 1;
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Left));
        assert_eq!(cursor, 0);
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Right));
        assert_eq!(cursor, 1);
    }

    #[test]
    fn test_home_end() {
        let mut buf = "first\nsecond".to_string();
        let mut cursor = 8; // middle of "second"
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Home));
        assert_eq!(cursor, 6); // start of "second"
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::End));
        assert_eq!(cursor, 12); // end of "second"
    }

    #[test]
    fn test_enter_newline() {
        let mut buf = "ab".to_string();
        let mut cursor = 1;
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Enter));
        assert_eq!(buf, "a\nb");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn test_cursor_line_col() {
        let buf = "first\nsecond\nthird";
        assert_eq!(cursor_line_col(buf, 0), (0, 0));
        assert_eq!(cursor_line_col(buf, 5), (0, 5));
        assert_eq!(cursor_line_col(buf, 6), (1, 0));
        assert_eq!(cursor_line_col(buf, 12), (1, 6));
        assert_eq!(cursor_line_col(buf, 13), (2, 0));
    }

    #[test]
    fn test_up_down_arrow() {
        let mut buf = "abc\ndef\nghi".to_string();
        let mut cursor = 1; // line 0, col 1
        // Down arrow -> line 1, col 1
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Down));
        assert_eq!(cursor_line_col(&buf, cursor), (1, 1));
        // Down arrow -> line 2, col 1
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Down));
        assert_eq!(cursor_line_col(&buf, cursor), (2, 1));
        // Up arrow -> line 1, col 1
        handle_text_input(&mut buf, &mut cursor, key(KeyCode::Up));
        assert_eq!(cursor_line_col(&buf, cursor), (1, 1));
    }

    #[test]
    fn test_text_editor_struct() {
        let mut editor = TextEditor::with_text("hello world");
        assert_eq!(editor.cursor, 11);
        assert_eq!(editor.line_count(), 1);

        editor.handle_input(key(KeyCode::Enter));
        assert_eq!(editor.text(), "hello world\n");
        assert_eq!(editor.line_count(), 2);
        assert_eq!(editor.cursor_position(), (1, 0));

        editor.clear();
        assert_eq!(editor.text(), "");
        assert_eq!(editor.cursor, 0);
    }
}
