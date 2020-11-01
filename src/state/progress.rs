use super::{ApplicationState, LogMessage, MessageType};

#[derive(PartialEq)]
pub enum ProgressState {
    Started(usize),               // id
    Working(usize, usize, usize), // id, file_size, current_bytes
    Stopped(usize),               // file_size
}

impl ApplicationState {
    pub fn progress_start(&mut self, id: usize) {
        self.messages.push(LogMessage::new(
            "Sending".into(),
            MessageType::Progress(ProgressState::Started(id)),
        ))
    }

    pub fn progress_pulse(&mut self, id: usize, file_size: usize, bytes_read: usize) {
        for msg in self.messages.iter_mut().rev() {
            match &msg.message_type {
                MessageType::Progress(ProgressState::Started(msg_id)) => {
                    if msg_id == &id {
                        msg.message_type = MessageType::Progress(ProgressState::Working(
                            id, file_size, bytes_read,
                        ));
                        break;
                    }
                }
                // same file_size
                MessageType::Progress(ProgressState::Working(msg_id, _, current_bytes)) => {
                    if msg_id == &id {
                        msg.message_type = MessageType::Progress(ProgressState::Working(
                            id,
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

    pub fn progress_stop(&mut self, id: usize) {
        for msg in self.messages.iter_mut().rev() {
            match &msg.message_type {
                MessageType::Progress(ProgressState::Started(msg_id)) => {
                    if msg_id == &id {
                        msg.message_type = MessageType::Progress(ProgressState::Stopped(0));
                        break;
                    }
                }
                // same file_size
                MessageType::Progress(ProgressState::Working(msg_id, _, current_bytes)) => {
                    if msg_id == &id {
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
