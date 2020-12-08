use crate::action::{Action, Processing};
use crate::commands::{Command};
use crate::state::{State};
use crate::message::{NetMessage};
use crate::util::{Result};

use message_io::network::{Network};

pub struct SsCommand;

impl Command for SsCommand {
    fn name(&self) -> &'static str {
        "ss"
    }

    fn parse_params(&self, _params: &[&str]) -> Result<Box<dyn Action>> {
        match Ss::new() {
            Ok(action) => Ok(Box::new(action)),
            Err(e) => Err(e),
        }
    }
}

pub struct Ss {
    stream: v4l::prelude::MmapStream<'static>,
}

use v4l::prelude::*;
use v4l::FourCC;
impl Ss {
    pub fn new() -> Result<Ss> {
        let mut dev = CaptureDevice::new(0).expect("Failed to open device");

        let mut fmt = dev.format().expect("Failed to read format");
        fmt.fourcc = FourCC::new(b"YUYV");
        dev.set_format(&fmt).expect("Failed to write format");

        let stream = MmapStream::with_buffers(&mut dev, 4).expect("Failed to create buffer stream");

        Ok(Ss { stream })
    }
}

impl Action for Ss {
    fn process(&mut self, state: &mut State, network: &mut Network) -> Processing {
        let message = NetMessage::S(self.stream.next().unwrap().data().to_vec());
        network.send_all(state.all_user_endpoints(), message);
        Processing::Partial
    }
}
