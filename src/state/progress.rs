use super::{ApplicationState, LogMessage, MessageType};

#[derive(PartialEq)]
pub enum ProgressState {
    Started(String),
    Working(String, usize, usize),
    Stopped(usize),
}

impl ApplicationState {
    // file name is used as a unique id to identify the progress
    // obviously its not really unique
    // but it shouldnt matter that much since sending the same file twice shouldnt be that frequent
    pub fn progress_start(&mut self, file_name: String) {
        self.messages.push(LogMessage::new(
            "Sending".into(),
            MessageType::Progress(ProgressState::Started(file_name)),
        ))
    }

    pub fn progress_pulse(&mut self, file_name: String, file_size: usize, bytes_read: usize) {
        for msg in self.messages.iter_mut().rev() {
            match &msg.message_type {
                MessageType::Progress(ProgressState::Started(name)) => {
                    if name == &file_name {
                        msg.message_type = MessageType::Progress(ProgressState::Working(
                            file_name, file_size, bytes_read,
                        ));
                        break;
                    }
                }
                // same file_size
                MessageType::Progress(ProgressState::Working(name, _, current_bytes)) => {
                    if name == &file_name {
                        msg.message_type = MessageType::Progress(ProgressState::Working(
                            file_name,
                            file_size,
                            current_bytes + bytes_read,
                        ));
                        break;
                    }
                }
                _ => (),
            }
        }
    }

    pub fn progress_stop(&mut self, file_name: String) {
        for msg in self.messages.iter_mut().rev() {
            match &msg.message_type {
                MessageType::Progress(ProgressState::Started(name)) => {
                    if name == &file_name {
                        msg.message_type = MessageType::Progress(ProgressState::Stopped(0));
                        break;
                    }
                }
                // same file_size
                MessageType::Progress(ProgressState::Working(name, _, current_bytes)) => {
                    if name == &file_name {
                        msg.message_type =
                            MessageType::Progress(ProgressState::Stopped(*current_bytes));
                        break;
                    }
                }
                _ => (),
            }
        }
    }

    pub fn progress_stop_last(&mut self) {
        for msg in self.messages.iter_mut().rev() {
            match &msg.message_type {
                MessageType::Progress(ProgressState::Started(_)) => {
                    msg.message_type = MessageType::Progress(ProgressState::Stopped(0));
                    break;
                }
                // same file_size
                MessageType::Progress(ProgressState::Working(_, _, current_bytes)) => {
                    msg.message_type =
                        MessageType::Progress(ProgressState::Stopped(*current_bytes));
                    break;
                }
                _ => (),
            }
        }
    }
}
