pub mod send_file;

use crate::action::{Action};
use crate::util::{Result};

use std::collections::{HashMap};

pub trait Command {
    fn name(&self) -> &'static str;
    fn parse_params(&self, params: Vec<&str>) -> Result<Box<dyn Action>>;
}

#[derive(Default)]
pub struct CommandManager {
    parsers: HashMap<&'static str, Box<dyn Command>>,
}

impl CommandManager {
    pub const COMMAND_PREFIX: &'static str = "?";
    pub fn with(mut self, command_parser: impl Command + 'static) -> Self {
        self.parsers.insert(command_parser.name(), Box::new(command_parser));
        self
    }

    pub fn find_command_action(&self, input: &str) -> Option<Result<Box<dyn Action>>> {
        let mut input = input.split_whitespace();
        let start = input.next().expect("Input must have some content");
        if start.starts_with(Self::COMMAND_PREFIX) {
            if let Some(parser) = self.parsers.get(&start[1..]) {
                return Some(parser.parse_params(input.collect()))
            }
        }
        None
    }
}
