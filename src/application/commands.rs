use super::Application;
use crate::state::ApplicationState;
use crate::util::Result;

impl Application {
    pub fn parse_input(&mut self, input: &str, state: &mut ApplicationState) -> Result<()> {
        const SEND_COMMAND: &str = "?send";
        if input.starts_with(SEND_COMMAND) {
            self.handle_send_command(input, state)?;
        }
        Ok(())
    }

    fn handle_send_command(&mut self, input: &str, state: &mut ApplicationState) -> Result<()> {
        const READ_FILENAME_ERROR: &str = "Unable to read file name";

        let path =
            std::path::Path::new(input.split_whitespace().nth(1).ok_or("No file specified")?);
        let file_name = path
            .file_name()
            .ok_or(READ_FILENAME_ERROR)?
            .to_str()
            .ok_or(READ_FILENAME_ERROR)?
            .to_string();

        let thread = self.read_file_ev
            .send(self.id, file_name, path.to_path_buf());
        self.send_threads.insert(self.id, thread);

        state.progress_start(self.id);
        self.id += 1;

        Ok(())
    }
}
