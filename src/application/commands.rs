use super::Event;
use super::{stringify_sendall_errors, Application, NetMessage};
use crate::state::{ApplicationState, TermchatMessageType};
use crate::ui;
use crate::util::{termchat_message, Result};
use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};

impl Application {
    pub fn parse_input(&mut self, input: &str, state: &mut ApplicationState) -> Result<()> {
        const SEND_COMMAND: &str = "?send";
        if input.starts_with(SEND_COMMAND) {
            self.handle_send_command(input, state)?;
        }
        Ok(())
    }

    fn handle_send_command(&mut self, input: &str, state: &mut ApplicationState) -> Result<()> {
        use std::io::Read;
        const READ_FILENAME_ERROR: &str = "Unable to read file name";

        let path =
            std::path::Path::new(input.split_whitespace().nth(1).ok_or("No file specified")?);
        let file_name = path
            .file_name()
            .ok_or(READ_FILENAME_ERROR)?
            .to_str()
            .ok_or(READ_FILENAME_ERROR)?
            .to_string();

        use std::convert::TryInto;
        let file_size = std::fs::metadata(path)?.len().try_into()?;
        state.progress.start(file_size);

        let mut file = std::fs::File::open(path)?;
        const BLOCK: usize = 65536;
        let mut data = [0; BLOCK];

        loop {
            match file.read(&mut data) {
                Ok(bytes_read) => {
                    state.progress.advance(bytes_read);
                    let data_to_send = data[..bytes_read].to_vec();

                    self.network
                        .send_all(
                            state.all_user_endpoints(),
                            NetMessage::UserData(
                                file_name.clone(),
                                Some((data_to_send, bytes_read)),
                                None,
                            ),
                        )
                        .map_err(stringify_sendall_errors)?;

                    // done
                    if bytes_read == 0 {
                        let msg = format!("Successfully sent file {} !", file_name);
                        let msg = termchat_message(msg, TermchatMessageType::Notification);
                        state.add_message(msg);
                        break;
                    }
                }
                Err(e) => {
                    state.progress.done();

                    self.network
                        .send_all(
                            state.all_user_endpoints(),
                            NetMessage::UserData(file_name, None, Some(e.to_string())),
                        )
                        .map_err(stringify_sendall_errors)?;
                    return Err(e.into());
                }
            }
            ui::draw(&mut self.terminal, &state)?;
            // check for ctrl_c
            if let Some(Event::Terminal(TermEvent::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }))) = self
                .event_queue
                .receive_event_timeout(std::time::Duration::from_millis(100))
            {
                let message =
                    termchat_message("File not sent.".into(), TermchatMessageType::Notification);
                state.add_message(message);

                self.network
                    .send_all(
                        state.all_user_endpoints(),
                        NetMessage::UserData(file_name, None, Some("User aborted.".into())),
                    )
                    .map_err(stringify_sendall_errors)?;
                break;
            }
        }
        state.progress.done();
        ui::draw(&mut self.terminal, &state)?;
        Ok(())
    }
}
