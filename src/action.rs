use crate::state::{State};
use crate::util::{Result};

use message_io::network::{NetworkManager};

pub enum Processing {
    Completed,
    Partial,
}

pub trait Action: Send {
    fn process(&mut self, state: &mut State, network: &mut NetworkManager) -> Result<Processing>;
}
