use crate::state::{State};

use message_io::network::{Network};

pub enum Processing {
    Completed,
    Partial,
}

pub trait Action: Send {
    fn process(&mut self, state: &mut State, network: &mut Network) -> Processing;
}
