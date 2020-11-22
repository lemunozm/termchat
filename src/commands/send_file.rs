use crate::commands::{Command, Action, Processing};
use crate::state::{State};
use crate::util::{Result};

use std::path::{Path};

pub struct SendFileCommand {

}

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
    //id: usize,
    file_name: String,
    //data: Vec<u8>,
    //bytes_read: usize,
    file_size: u64,
}

impl SendFile {
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
            file_size
        })
    }
}

impl Action for SendFile {
    fn process(&mut self, state: &mut State) -> Result<Processing> {
        /*

        let send_id = self.id;
        (self.callback)(file, file_name, file_size, send_id);
        self.id += 1;

        self.state.progress_start(send_id);
        */
        Err("".into())
    }
}
