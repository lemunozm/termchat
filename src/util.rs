// split messages to fit the width of the ui panel
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
pub fn split_each(input: String, width: usize) -> Vec<String> {
    let mut splitted = Vec::with_capacity(input.width() / width);
    let mut row = String::new();

    let mut index = 0;

    for current_char in input.chars() {
        if (index != 0 && index == width) || index + current_char.width().unwrap_or(0) > width {
            splitted.push(row.drain(..).collect());
            index = 0;
        }

        row.push(current_char);
        index += current_char.width().unwrap_or(0);
    }
    // leftover
    if !row.is_empty() {
        splitted.push(row.drain(..).collect());
    }
    splitted
}

// Termchat messages convenience function
use crate::state::{LogMessage, MessageType, TermchatMessageType};
pub fn termchat_message(content: String, msg_type: TermchatMessageType) -> LogMessage {
    LogMessage::new(
        "Termchat: ".into(),
        MessageType::Termchat(content, msg_type),
    )
}

// Errors
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub fn stringify_sendall_errors(e: Vec<(message_io::network::Endpoint, std::io::Error)>) -> String {
    let mut out = String::new();
    for (endpoint, error) in e {
        let msg = format!("Failed to connect to {}, error: {}", endpoint, error);
        out.push_str(&msg);
        out.push('\n');
    }
    // remove last new line
    if !out.is_empty() {
        out.pop();
    }
    out
}

// Progress bar handling
#[derive(Default)]
pub struct Progress {
    pub max: usize,
    pub current: usize,
    pub state: ProgressState,
}

#[derive(PartialEq)]
pub enum ProgressState {
    Idle,
    Working,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self::Idle
    }
}

impl Progress {
    pub fn start(&mut self, max: usize) {
        debug_assert!(self.state == ProgressState::Idle);
        self.max = max;
        self.state = ProgressState::Working;
    }

    pub fn advance(&mut self, n: usize) {
        debug_assert!(self.state == ProgressState::Working);
        self.current += n;
    }

    pub fn done(&mut self) {
        debug_assert!(self.state == ProgressState::Working);
        *self = Self::default();
    }
}
