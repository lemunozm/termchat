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
    width: usize,
    height: usize,
}

use v4l::prelude::*;
use v4l::FourCC;
impl Ss {
    pub fn new() -> Result<Ss> {
        let mut dev = CaptureDevice::new(0).expect("Failed to open device");

        let mut fmt = dev.format().expect("Failed to read format");
        fmt.fourcc = FourCC::new(b"YUYV");
        let width = fmt.width as usize;
        let height = fmt.height as usize;
        dev.set_format(&fmt).expect("Failed to write format");

        let stream = MmapStream::with_buffers(&mut dev, 4).expect("Failed to create buffer stream");

        Ok(Ss { stream, width, height })
    }
}

use byteorder::ByteOrder;
impl Action for Ss {
    fn process(&mut self, state: &mut State, network: &mut Network) -> Processing {
        if state.x == crate::state::Xstate::Idle {
            return Processing::Completed
        }
        let data = self
            .stream
            .next()
            .unwrap()
            .data()
            .chunks(4)
            .map(|v| {
                let v = crate::util::yuyv_to_rgb(v);
                byteorder::BigEndian::read_u32(&v)
            })
            .collect();

        let message = NetMessage::S(Some((data, self.width, self.height)));
        network.send_all(state.all_user_endpoints(), message);
        Processing::Partial
    }
}
