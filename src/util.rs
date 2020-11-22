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
