use message_io::network::Endpoint;

use chrono::{DateTime, Local};

use std::collections::HashMap;

pub enum MessageType {
    Connection,
    Disconnection,
    Content(String),
}

pub struct LogMessage {
    pub date: DateTime<Local>,
    pub user: String,
    pub message_type: MessageType,
}

impl LogMessage {
    pub fn new(user: String, message_type: MessageType) -> LogMessage {
        LogMessage {
            date: Local::now(),
            user,
            message_type,
        }
    }
}

pub struct ApplicationState {
    messages: Vec<LogMessage>,
    scroll_messages_view: usize,
    input: String,
    input_cursor: usize,
    lan_users: HashMap<Endpoint, String>,
    users_id: HashMap<String, usize>,
    last_user_id: usize,
}

pub enum CursorMovement {
    Left,
    Right,
    Start,
    End,
}

pub enum ScrollMovement {
    Up,
    Down,
    Start,
}

impl ApplicationState {
    pub fn new() -> ApplicationState {
        ApplicationState {
            messages: Vec::new(),
            scroll_messages_view: 0,
            input: String::new(),
            input_cursor: 0,
            lan_users: HashMap::new(),
            users_id: HashMap::new(),
            last_user_id: 0,
        }
    }

    pub fn messages(&self) -> &Vec<LogMessage> {
        &self.messages
    }

    pub fn scroll_messages_view(&self) -> usize {
        self.scroll_messages_view
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn input_cursor(&self) -> usize {
        self.input_cursor
    }

    pub fn user_name(&self, endpoint: Endpoint) -> Option<&String> {
        self.lan_users.get(&endpoint)
    }

    pub fn all_user_endpoints(&self) -> impl Iterator<Item = &Endpoint> {
        self.lan_users.keys()
    }

    pub fn users_id(&self) -> &HashMap<String, usize> {
        &self.users_id
    }

    pub fn connected_user(&mut self, endpoint: Endpoint, user: &str) {
        self.lan_users.insert(endpoint, user.into());
        if !self.users_id.contains_key(user) {
            self.users_id.insert(user.into(), self.last_user_id);
        }
        self.last_user_id += 1;
        self.add_message(LogMessage::new(user.into(), MessageType::Connection));
    }

    pub fn disconnected_user(&mut self, endpoint: Endpoint) {
        let user = self.lan_users.remove(&endpoint).unwrap();
        self.add_message(LogMessage::new(user, MessageType::Disconnection));
    }

    pub fn input_write(&mut self, character: char) {
        self.input.insert(self.input_cursor, character);
        self.input_cursor += 1;
    }

    pub fn input_remove(&mut self) {
        if self.input_cursor < self.input.len() {
            self.input.remove(self.input_cursor);
        }
    }

    pub fn input_remove_previous(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor -= 1;
            self.input.remove(self.input_cursor);
        }
    }

    pub fn input_move_cursor(&mut self, movement: CursorMovement) {
        match movement {
            CursorMovement::Left => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                }
            }
            CursorMovement::Right => {
                if self.input_cursor < self.input.len() {
                    self.input_cursor += 1;
                }
            }
            CursorMovement::Start => {
                self.input_cursor = 0;
            }
            CursorMovement::End => {
                self.input_cursor = self.input.len();
            }
        }
    }

    pub fn messages_scroll(&mut self, movement: ScrollMovement) {
        match movement {
            ScrollMovement::Up => {
                if self.scroll_messages_view > 0 {
                    self.scroll_messages_view -= 1;
                }
            }
            ScrollMovement::Down => {
                self.scroll_messages_view += 1;
            }
            ScrollMovement::Start => {
                self.scroll_messages_view += 0;
            }
        }
    }

    pub fn reset_input(&mut self) -> Option<String> {
        if self.input.len() > 0 {
            self.input_cursor = 0;
            return Some(self.input.drain(..).collect());
        }
        None
    }

    pub fn add_message(&mut self, message: LogMessage) {
        self.messages.push(message);
    }
}
