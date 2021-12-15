use crate::state::{State};

use message_io::network::{NetworkController};

use std::time::{Duration};

pub enum Processing {
    Completed,
    Partial(Duration),
}

pub trait Action: Send {
    fn process(&mut self, state: &mut State, network: &NetworkController) -> Processing;
}
