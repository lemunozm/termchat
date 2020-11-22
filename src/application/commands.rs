use super::Application;
use crate::util::Result;

impl Application<'_> {
    pub fn parse_input(&mut self, input: &str) -> Result<()> {
        const SEND_COMMAND: &str = "?send";
        if input.starts_with(SEND_COMMAND) {
            self.handle_send_command(input)?;
        }
        Ok(())
    }

    fn handle_send_command(&mut self, input: &str) -> Result<()> {
        const READ_FILENAME_ERROR: &str = "Unable to read file name";

        let path =
            std::path::Path::new(input.split_whitespace().nth(1).ok_or("No file specified")?);
        let file_name = path
            .file_name()
            .ok_or(READ_FILENAME_ERROR)?
            .to_str()
            .ok_or(READ_FILENAME_ERROR)?
            .to_string();

        let send_id = self.read_file_ev.send(file_name, path.to_path_buf())?;
        self.state.progress_start(send_id);

        Ok(())
    }
}
