pub mod send_file;

use crate::action::{Action};
use crate::util::{Result};

use std::collections::{HashMap};

pub trait Command {
    fn name(&self) -> &'static str;
    fn parse_params(&self, params: &[&str]) -> Result<Box<dyn Action>>;
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
            let params = input.split_whitespace().collect::<Vec<_>>();
            if let Some((first, rest)) = params.split_first() {
                if let Some(parser) = self.parsers.get(first) {
                    return Some(parser.parse_params(rest))
                }
            }
        }
        None
    }
}
