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

//TODO: Should send the file even if some endpoint of send_all gives an error.
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

use crate::state::State;
/// Trait for reporting Recoverable errors/ Infos to the user
pub trait Reportable: Sized {
    fn report_if_err(self, _state: &mut State) {
        unimplemented!()
    }
    fn report_err(self, _state: &mut State) {
        unimplemented!()
    }
    fn report_info(self, _state: &mut State) {
        unimplemented!()
    }
    fn report_warn(self, _state: &mut State) {
        unimplemented!()
    }
}

impl Reportable for Result<()> {
    fn report_if_err(self, state: &mut State) {
        if let Err(e) = self {
            state.add_system_error_message(e.to_string());
        }
    }
}

impl Reportable for std::result::Result<(), Vec<(message_io::network::Endpoint, std::io::Error)>> {
    fn report_if_err(self, state: &mut State) {
        if let Err(e) = self {
            state.add_system_error_message(crate::util::stringify_sendall_errors(e));
        }
    }
}

impl Reportable for Box<dyn std::error::Error + Send + Sync> {
    fn report_err(self, state: &mut State) {
        self.to_string().report_err(state);
    }
}

impl Reportable for String {
    fn report_err(self, state: &mut State) {
        state.add_system_error_message(self);
    }

    fn report_info(self, state: &mut State) {
        state.add_system_info_message(self);
    }

    fn report_warn(self, state: &mut State) {
        state.add_system_info_message(self);
    }
}
