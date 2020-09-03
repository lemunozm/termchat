use chrono::{DateTime, Local};

pub struct UserMessage {
   pub user: String,
   pub msg: String,
   pub date: DateTime<Local>,
}

pub struct ApplicationState {
    messages: Vec<UserMessage>,
    scroll_messages_view: usize,
    input: String,
    input_cursor: usize,
}

pub enum CursorMovement {
   Left, Right, Start, End,
}

pub enum ScrollMovement {
   Up, Down, Start,
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

    pub fn messages(&self) -> &Vec<UserMessage> {
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
            },
            CursorMovement::Right => {
                if self.input_cursor < self.input.len() {
                    self.input_cursor += 1;
                }
            },
            CursorMovement::Start => {
                self.input_cursor = 0;
            },
            CursorMovement::End => {
                self.input_cursor = self.input.len();
            },
        }
    }

    pub fn messages_scroll(&mut self, movement: ScrollMovement) {
        match movement {
            ScrollMovement::Up => {
                if self.scroll_messages_view > 0 {
                    self.scroll_messages_view -= 1;
                }
            },
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
            return Some(self.input.drain(..).collect())
        }
        None
    }

    pub fn add_message(&mut self, message: UserMessage) {
        self.messages.push(message);
    }
}
