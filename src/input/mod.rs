pub mod key;
pub mod mouse;
pub mod text_editor;

pub use key::handle_key_event;
pub use mouse::handle_mouse_event;
pub use text_editor::TextEditor;
