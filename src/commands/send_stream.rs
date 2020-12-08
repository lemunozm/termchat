use crate::action::{Action, Processing};
use crate::commands::{Command};
use crate::state::{State};
use crate::message::{NetMessage};
use crate::util::{Result};

use message_io::network::{Network};

pub struct SendStreamCommand;

impl Command for SendStreamCommand {
    fn name(&self) -> &'static str {
        "stream"
    }

    fn parse_params(&self, _params: &[&str]) -> Result<Box<dyn Action>> {
        match SendStream::new() {
            Ok(action) => Ok(Box::new(action)),
            Err(e) => Err(e),
        }
    }
}

pub struct SendStream {
    stream: v4l::prelude::MmapStream<'static>,
    width: usize,
    height: usize,
}

use v4l::prelude::*;
use v4l::FourCC;
impl SendStream {
    pub fn new() -> Result<SendStream> {
        let mut dev = CaptureDevice::new(0).expect("Failed to open device");

        let mut fmt = dev.format().expect("Failed to read format");
        fmt.fourcc = FourCC::new(b"YUYV");
        let width = fmt.width as usize;
        let height = fmt.height as usize;
        dev.set_format(&fmt).expect("Failed to write format");

        let stream = MmapStream::with_buffers(&mut dev, 4).expect("Failed to create buffer stream");

        Ok(SendStream { stream, width, height })
    }
}

use byteorder::ByteOrder;
impl Action for SendStream {
    fn process(&mut self, state: &mut State, network: &mut Network) -> Processing {
        if state.x == crate::state::Xstate::Idle {
            network.send_all(state.all_user_endpoints(), NetMessage::Stream(None));
            return Processing::Completed;
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

        let message = NetMessage::Stream(Some((data, self.width, self.height)));
        network.send_all(state.all_user_endpoints(), message);
        Processing::Partial
    }
}
