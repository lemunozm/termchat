use crate::commands::{Command, Action};
use crate::util::Result;

pub struct SendStreamCommand; //unimplemented
pub struct StopStreamCommand; //unimplemented

impl Command for SendStreamCommand {
    fn name(&self) -> &'static str {
        "stream"
    }

    fn parse_params(&self, _param: Option<&str>) -> Result<Box<dyn Action>> {
        Err(format!("{} command is not supported on this platform.", self.name()).into())
    }
}

impl Command for StopStreamCommand {
    fn name(&self) -> &'static str {
        "stopstream"
    }

    fn parse_params(&self, _param: Option<&str>) -> Result<Box<dyn Action>> {
        Err(format!("{} command is not supported on this platform.", self.name()).into())
    }
}
