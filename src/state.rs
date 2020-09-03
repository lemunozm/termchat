use chrono::{DateTime, Local};

pub struct UserMessage {
   pub user: String,
   pub data: String,
   pub date: DateTime<Local>,
}

pub struct ApplicationState {
    pub messages: Vec<UserMessage>,
    pub scroll_messages_view: usize,
    pub input: String,
    pub input_cursor: usize,
}

impl ApplicationState {
    pub fn new() -> ApplicationState {
        ApplicationState {
            messages: Vec::new(),
            scroll_messages_view: 0,
            input: String::new(),
            input_cursor: 0,
        }
    }
}
