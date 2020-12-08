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

use byteorder::ByteOrder;
impl Action for Ss {
    fn process(&mut self, state: &mut State, network: &mut Network) -> Processing {
        if state.x == crate::state::Xstate::Idle {
            return Processing::Completed;
        }
        #[allow(non_snake_case)]
        let data = self
            .stream
            .next()
            .unwrap()
            .data()
            .chunks(4)
            .map(|v| {
                // convert form YUYV to RGB
                let [Y, U, _, V]: [u8; 4] = std::convert::TryFrom::try_from(v).unwrap();
                let Y = Y as f32;
                let U = U as f32;
                let V = V as f32;

                let B = 1.164 * (Y - 16.) + 2.018 * (U - 128.);

                let G = 1.164 * (Y - 16.) - 0.813 * (V - 128.) - 0.391 * (U - 128.);

                let R = 1.164 * (Y - 16.) + 1.596 * (V - 128.);
                let v = [0, R as u8, G as u8, B as u8];
                byteorder::BigEndian::read_u32(&v)
            })
            .collect();

        let message = NetMessage::S(Some(data));
        network.send_all(state.all_user_endpoints(), message);
        Processing::Partial
    }
}
