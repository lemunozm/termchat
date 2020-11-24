use crate::state::{State};

use message_io::network::{NetworkManager};

pub enum Processing {
    Completed,
    Partial,
}

pub trait Action: Send {
    fn process(&mut self, state: &mut State, network: &mut NetworkManager) -> Processing;
}
