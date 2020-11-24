use crate::action::{Action, Processing};
use crate::commands::{Command};
use crate::state::{State};
use crate::message::{NetMessage, Chunk};
use crate::util::{Result};

use message_io::network::{NetworkManager};

use std::path::{Path};
use std::io::{Read};

pub struct SendFileCommand;

impl Command for SendFileCommand {
    fn name(&self) -> &'static str {
        "send"
    }

    fn parse_params(&self, params: Vec<&str>) -> Result<Box<dyn Action>> {
        let file_path = params.get(0).ok_or("No file specified")?;
        match SendFile::new(file_path) {
            Ok(action) => Ok(Box::new(action)),
            Err(e) => Err(e),
        }
    }
}


pub struct SendFile {
    file: std::fs::File,
    file_name: String,
    file_size: u64,
    progress_id: Option<usize>,
}

impl SendFile {
    const CHUNK_SIZE: usize = 65500;

    pub fn new(file_path: &str) -> Result<SendFile> {
        const READ_FILENAME_ERROR: &str = "Unable to read file name";
        let file_path = Path::new(file_path);
        let file_name = file_path
            .file_name()
            .ok_or(READ_FILENAME_ERROR)?
            .to_str()
            .ok_or(READ_FILENAME_ERROR)?
            .to_string();

        let file_size = std::fs::metadata(file_path)?.len();
        let file = std::fs::File::open(file_path)?;

        Ok(SendFile {
            file,
            file_name,
            file_size,
            progress_id: None,
        })
    }
}

impl Action for SendFile {
    fn process(&mut self, state: &mut State, network: &mut NetworkManager) -> Processing {
        if self.progress_id.is_none() {
            let id = state.add_progress_message(&self.file_name, self.file_size);
            self.progress_id = Some(id);
        }

        let mut data = [0; Self::CHUNK_SIZE];
        let (bytes_read, chunk, processing) = match self.file.read(&mut data) {
            Ok(0) => {
                (0, Chunk::End, Processing::Completed)
            }
            Ok(bytes_read) => {
                (bytes_read, Chunk::Data(data.to_vec()), Processing::Partial)
            }
            Err(error) => {
                let msg = format!("Error sending file. error: {}", error);
                state.add_system_error_message(msg);
                (0, Chunk::Error, Processing::Completed)
            }
        };

        state.progress_message_update(self.progress_id.unwrap(), bytes_read as u64);

        let message = NetMessage::UserData(self.file_name.clone(), chunk);
        network.send_all(state.all_user_endpoints(), message).ok(); //Best effort

        processing
    }
}
