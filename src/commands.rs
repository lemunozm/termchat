pub mod send_file;
#[cfg(feature = "stream-video")]
pub mod send_stream;
pub mod send_audio;

use crate::action::{Action};
use crate::util::{Result};

use std::collections::{HashMap};

pub trait Command {
    fn name(&self) -> &'static str;
    fn parse_params(&self, params: Vec<String>) -> Result<Box<dyn Action>>;
}

#[derive(Default)]
pub struct CommandManager {
    parsers: HashMap<&'static str, Box<dyn Command + Send>>,
}

impl CommandManager {
    pub const COMMAND_PREFIX: &'static str = "?";

    pub fn with(mut self, command_parser: impl Command + 'static + Send) -> Self {
        self.parsers.insert(command_parser.name(), Box::new(command_parser));
        self
    }

    pub fn find_command_action(&self, input: &str) -> Option<Result<Box<dyn Action>>> {
        if let Some(input) = input.strip_prefix(Self::COMMAND_PREFIX) {
            let mut input = input.splitn(2, char::is_whitespace);
            if let Some(first) = input.next() {
                if let Some(parser) = self.parsers.get(first) {
                    let param_str = input.next().unwrap_or("");
                    return match shellwords::split(param_str) {
                        Ok(params) => Some(parser.parse_params(params)),
                        Err(err) => Some(Err(err.into())),
                    }
                }
            }
        }
        None
    }
}
