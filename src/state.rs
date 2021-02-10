use message_io::network::Endpoint;
use chrono::{DateTime, Local};

use std::collections::HashMap;

#[derive(PartialEq)]
pub enum SystemMessageType {
    Info,
    Warning,
    Error,
}

#[derive(PartialEq)]
pub enum ProgressState {
    Started(u64),      // file_size
    Working(u64, u64), // file_size, current_bytes
    Completed,
}

pub enum MessageType {
    Connection,
    Disconnection,
    Text(String),
    System(String, SystemMessageType),
    Progress(ProgressState),
}

pub struct ChatMessage {
    pub date: DateTime<Local>,
    pub user: String,
    pub message_type: MessageType,
}

impl ChatMessage {
    pub fn new(user: String, message_type: MessageType) -> ChatMessage {
        ChatMessage { date: Local::now(), user, message_type }
    }
}

pub struct Window {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}
impl Window {
    pub fn new(width: usize, height: usize) -> Self {
        Self { data: vec![], width, height }
    }
}
#[derive(Default)]
pub struct State {
    messages: Vec<ChatMessage>,
    scroll_messages_view: usize,
    input: Vec<char>,
    input_cursor: usize,
    lan_users: HashMap<Endpoint, String>,
    users_id: HashMap<String, usize>,
    last_user_id: usize,
    pub stop_stream: bool,
    pub windows: HashMap<Endpoint, Window>,
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

impl State {
    pub fn messages(&self) -> &Vec<ChatMessage> {
        &self.messages
    }

    pub fn scroll_messages_view(&self) -> usize {
        self.scroll_messages_view
    }

    pub fn input(&self) -> &[char] {
        &self.input
    }

    pub fn ui_input_cursor(&self, width: usize) -> (u16, u16) {
        let mut position = (0, 0);

        for current_char in self.input.iter().take(self.input_cursor) {
            let char_width = unicode_width::UnicodeWidthChar::width(*current_char).unwrap_or(0);

            position.0 += char_width;

            match position.0.cmp(&width) {
                std::cmp::Ordering::Equal => {
                    position.0 = 0;
                    position.1 += 1;
                }
                std::cmp::Ordering::Greater => {
                    // Handle a char with width > 1 at the end of the row
                    // width - (char_width - 1) accounts for the empty column(s) left behind
                    position.0 -= width - (char_width - 1);
                    position.1 += 1;
                }
                _ => (),
            }
        }

        (position.0 as u16, position.1 as u16)
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
        self.add_message(ChatMessage::new(user.into(), MessageType::Connection));
    }

    pub fn disconnected_user(&mut self, endpoint: Endpoint) {
        if self.lan_users.contains_key(&endpoint) {
            // unwrap is safe because of the check above
            let user = self.lan_users.remove(&endpoint).unwrap();
            self.add_message(ChatMessage::new(user, MessageType::Disconnection));
        }
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
        if !self.input.is_empty() {
            self.input_cursor = 0;
            return Some(self.input.drain(..).collect())
        }
        None
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn add_system_warn_message(&mut self, content: String) {
        let message_type = MessageType::System(content, SystemMessageType::Warning);
        let message = ChatMessage::new("Termchat: ".into(), message_type);
        self.messages.push(message);
    }

    pub fn add_system_info_message(&mut self, content: String) {
        let message_type = MessageType::System(content, SystemMessageType::Info);
        let message = ChatMessage::new("Termchat: ".into(), message_type);
        self.messages.push(message);
    }

    pub fn add_system_error_message(&mut self, content: String) {
        let message_type = MessageType::System(content, SystemMessageType::Error);
        let message = ChatMessage::new("Termchat: ".into(), message_type);
        self.messages.push(message);
    }

    pub fn add_progress_message(&mut self, file_name: &str, total: u64) -> usize {
        let message = ChatMessage::new(
            format!("Sending '{}'", file_name),
            MessageType::Progress(ProgressState::Started(total)),
        );
        self.messages.push(message);
        self.messages.len() - 1
    }

    pub fn progress_message_update(&mut self, index: usize, increment: u64) {
        match &mut self.messages[index].message_type {
            MessageType::Progress(ref mut state) => {
                *state = match state {
                    ProgressState::Started(total) => ProgressState::Working(*total, increment),
                    ProgressState::Working(total, current) => {
                        let new_current = *current + increment;
                        if new_current == *total {
                            ProgressState::Completed
                        }
                        else {
                            ProgressState::Working(*total, new_current)
                        }
                    }
                    ProgressState::Completed => ProgressState::Completed,
                };
            }
            _ => panic!("Must be a Progress MessageType"),
        }
    }

    pub fn update_window(
        &mut self,
        endpoint: &Endpoint,
        data: Vec<u8>,
        width: usize,
        height: usize,
    )
    {
        let window = self.windows.get_mut(endpoint).expect("Window should exist");
        window.width = width;
        window.height = height;
        window.data = data;
    }
}
